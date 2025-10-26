//! TLB Shootdown Implementation
//!
//! This module implements TLB (Translation Lookaside Buffer) shootdown for
//! multicore systems. When page tables are modified, all CPUs that might have
//! cached translations must flush their TLBs to ensure memory consistency.

extern crate alloc;

use crate::arch::x86_64::apic::ipi::{send_ipi, TLB_FLUSH_IPI_VECTOR};
use crate::arch::x86_64::smp::percpu::{percpu_current, percpu_for};
use crate::arch::x86_64::smp::{get_cpu_count, is_cpu_online};
use crate::config::MAX_CPUS;
use crate::sync::SpinLock;
use crate::user::process::ProcessId;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Maximum number of CPUs that can be targeted in a single shootdown
const MAX_SHOOTDOWN_CPUS: usize = 64;

/// Maximum number of processes to track per CPU
const MAX_TRACKED_PROCESSES: usize = 64;

/// Per-CPU process tracking
///
/// Tracks which processes have been accessed by each CPU.
/// This allows us to optimize TLB shootdowns by only sending IPIs
/// to CPUs that have actually accessed the process.
struct CpuProcessTracker {
    /// Bitmask of process IDs that have been accessed by this CPU
    /// We use a simple array of u64 to track up to 64 processes
    process_mask: [AtomicU64; MAX_TRACKED_PROCESSES / 64],
}

impl CpuProcessTracker {
    const fn new() -> Self {
        const INIT: AtomicU64 = AtomicU64::new(0);
        Self {
            process_mask: [INIT; MAX_TRACKED_PROCESSES / 64],
        }
    }

    /// Mark a process as accessed by this CPU
    fn mark_accessed(&self, pid: ProcessId) {
        if pid < MAX_TRACKED_PROCESSES {
            let word_idx = pid / 64;
            let bit_idx = pid % 64;
            self.process_mask[word_idx].fetch_or(1u64 << bit_idx, Ordering::Relaxed);
        }
    }

    /// Check if a process has been accessed by this CPU
    fn has_accessed(&self, pid: ProcessId) -> bool {
        if pid < MAX_TRACKED_PROCESSES {
            let word_idx = pid / 64;
            let bit_idx = pid % 64;
            (self.process_mask[word_idx].load(Ordering::Relaxed) & (1u64 << bit_idx)) != 0
        } else {
            false
        }
    }

    /// Clear tracking for a process (when process exits)
    fn clear_process(&self, pid: ProcessId) {
        if pid < MAX_TRACKED_PROCESSES {
            let word_idx = pid / 64;
            let bit_idx = pid % 64;
            self.process_mask[word_idx].fetch_and(!(1u64 << bit_idx), Ordering::Relaxed);
        }
    }
}

/// Global CPU-to-Process mapping
///
/// Tracks which CPUs have accessed which processes for optimized TLB shootdowns.
static CPU_PROCESS_MAP: [CpuProcessTracker; MAX_CPUS] = {
    const INIT: CpuProcessTracker = CpuProcessTracker::new();
    [INIT; MAX_CPUS]
};

/// Batch TLB shootdown request
///
/// Accumulates multiple TLB flush requests within a time window
/// to reduce IPI overhead.
struct BatchedShootdown {
    /// Start address of the range to flush
    start_addr: AtomicUsize,
    /// End address of the range to flush
    end_addr: AtomicUsize,
    /// Timestamp of the first request in this batch (TSC)
    first_request_tsc: AtomicU64,
    /// Number of requests in this batch
    request_count: AtomicUsize,
}

impl BatchedShootdown {
    const fn new() -> Self {
        Self {
            start_addr: AtomicUsize::new(0),
            end_addr: AtomicUsize::new(0),
            first_request_tsc: AtomicU64::new(0),
            request_count: AtomicUsize::new(0),
        }
    }

    /// Try to add a request to this batch
    ///
    /// Returns true if the request was added, false if the batch should be flushed first
    fn try_add(&self, start: usize, end: usize, current_tsc: u64) -> bool {
        let first_tsc = self.first_request_tsc.load(Ordering::Acquire);

        // If this is the first request, initialize the batch
        if first_tsc == 0 {
            self.start_addr.store(start, Ordering::Release);
            self.end_addr.store(end, Ordering::Release);
            self.first_request_tsc.store(current_tsc, Ordering::Release);
            self.request_count.store(1, Ordering::Release);
            return true;
        }

        // Check if batch window has expired (1ms = ~2.4M TSC cycles at 2.4GHz)
        const TSC_PER_MS: u64 = 2_400_000;
        if current_tsc - first_tsc > TSC_PER_MS {
            return false; // Batch expired, flush it
        }

        // Expand the range to include this request
        let current_start = self.start_addr.load(Ordering::Acquire);
        let current_end = self.end_addr.load(Ordering::Acquire);

        let new_start = core::cmp::min(current_start, start);
        let new_end = core::cmp::max(current_end, end);

        self.start_addr.store(new_start, Ordering::Release);
        self.end_addr.store(new_end, Ordering::Release);
        self.request_count.fetch_add(1, Ordering::Release);

        true
    }

    /// Get the current batch range and reset
    fn take(&self) -> Option<(usize, usize, usize)> {
        let count = self.request_count.swap(0, Ordering::AcqRel);
        if count == 0 {
            return None;
        }

        let start = self.start_addr.load(Ordering::Acquire);
        let end = self.end_addr.load(Ordering::Acquire);

        // Reset for next batch
        self.first_request_tsc.store(0, Ordering::Release);

        Some((start, end, count))
    }
}

/// Per-CPU batched shootdown state
static BATCHED_SHOOTDOWNS: [BatchedShootdown; MAX_CPUS] = {
    const INIT: BatchedShootdown = BatchedShootdown::new();
    [INIT; MAX_CPUS]
};

/// TLB shootdown request structure
///
/// Contains information about a TLB flush request that needs to be
/// propagated to other CPU cores.
#[derive(Debug)]
pub struct TlbShootdownRequest {
    /// Starting virtual address to flush
    pub start_addr: usize,
    /// Ending virtual address to flush (exclusive)
    pub end_addr: usize,
    /// Number of CPUs that have acknowledged
    pub ack_count: AtomicUsize,
}

impl TlbShootdownRequest {
    /// Create a new TLB shootdown request
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        Self {
            start_addr,
            end_addr,
            ack_count: AtomicUsize::new(0),
        }
    }

    /// Check if all CPUs have acknowledged
    pub fn is_complete(&self, expected_count: usize) -> bool {
        self.ack_count.load(Ordering::Acquire) >= expected_count
    }

    /// Increment acknowledgment counter
    pub fn acknowledge(&self) {
        self.ack_count.fetch_add(1, Ordering::Release);
    }
}

/// Global TLB shootdown state
///
/// This structure coordinates TLB shootdowns across multiple CPUs.
static TLB_SHOOTDOWN: SpinLock<Option<TlbShootdownRequest>> = SpinLock::new(None);

/// Sequence number for TLB shootdowns (for debugging)
static TLB_SHOOTDOWN_SEQ: AtomicU64 = AtomicU64::new(0);

/// Flush a single page from the TLB
///
/// # Arguments
/// * `vaddr` - Virtual address of the page to flush
///
/// # Safety
/// This function uses the `invlpg` instruction which is safe but requires
/// the address to be properly aligned.
#[inline]
pub unsafe fn flush_page(vaddr: usize) {
    core::arch::asm!(
        "invlpg [{}]",
        in(reg) vaddr,
        options(nostack, preserves_flags)
    );
}

/// Flush the entire TLB
///
/// This reloads CR3 which flushes all non-global TLB entries.
///
/// # Safety
/// This function reloads CR3 which is safe but affects all address translations.
#[inline]
pub unsafe fn flush_all() {
    let cr3: usize;
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) cr3,
        options(nostack, preserves_flags)
    );
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) cr3,
        options(nostack, preserves_flags)
    );
}

/// Flush a range of pages from the TLB
///
/// # Arguments
/// * `vaddr` - Starting virtual address
/// * `page_count` - Number of pages to flush
///
/// # Safety
/// This function flushes TLB entries which affects address translation.
pub unsafe fn flush_range(vaddr: usize, page_count: usize) {
    const PAGE_SIZE: usize = 4096;

    // If flushing more than 64 pages, just flush the entire TLB
    if page_count > 64 {
        flush_all();
        return;
    }

    // Flush each page individually
    for i in 0..page_count {
        let addr = vaddr + (i * PAGE_SIZE);
        flush_page(addr);
    }
}

/// Send TLB shootdown IPI to a specific CPU
///
/// This function sends a TLB flush IPI to the specified CPU using the APIC IPI mechanism.
///
/// # Arguments
/// * `cpu_id` - Logical CPU ID to send IPI to
/// * `request` - Reference to the shootdown request (for logging/debugging)
///
/// # Returns
/// `true` if IPI was sent successfully, `false` otherwise
pub fn send_tlb_shootdown_ipi(cpu_id: usize, _request: &TlbShootdownRequest) -> bool {
    // Skip if CPU is not online
    if !is_cpu_online(cpu_id) {
        return false;
    }

    // Get target CPU's APIC ID
    let target_apic_id = percpu_for(cpu_id).apic_id;

    // Send TLB flush IPI
    send_ipi(target_apic_id, TLB_FLUSH_IPI_VECTOR)
}

/// Send TLB shootdown IPI to specified CPUs
///
/// This function sends TLB flush IPIs to all CPUs in the given mask,
/// excluding the current CPU.
///
/// # Arguments
/// * `cpu_mask` - Bitmask of CPUs to send IPI to
///
/// # Returns
/// Number of IPIs successfully sent
fn send_tlb_shootdown_ipis(cpu_mask: u64) -> usize {
    let current_cpu = percpu_current().id;
    let cpu_count = get_cpu_count();
    let mut sent_count = 0;

    for cpu_id in 0..cpu_count.min(64) {
        // Skip if this CPU is not in the mask
        if (cpu_mask & (1 << cpu_id)) == 0 {
            continue;
        }

        // Skip current CPU
        if cpu_id == current_cpu {
            continue;
        }

        // Skip if CPU is not online
        if !is_cpu_online(cpu_id) {
            continue;
        }

        // Get target CPU's APIC ID
        let target_apic_id = percpu_for(cpu_id).apic_id;

        // Send TLB flush IPI
        if send_ipi(target_apic_id, TLB_FLUSH_IPI_VECTOR) {
            sent_count += 1;
        }
    }

    sent_count
}

/// Wait for TLB shootdown acknowledgments
///
/// Spins until all expected CPUs have acknowledged the TLB flush.
///
/// # Arguments
/// * `request` - The shootdown request to wait for
/// * `expected_count` - Number of acknowledgments to wait for
///
/// # Returns
/// `true` if all acknowledgments received, `false` on timeout
///
/// # Notes
/// This function uses a 100ms timeout to prevent indefinite hangs.
/// If a timeout occurs, it indicates a serious problem with IPI delivery
/// or a CPU that has stopped responding.
pub fn wait_for_acks(request: &TlbShootdownRequest, expected_count: usize) -> bool {
    // Get current timestamp
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    const TSC_PER_MS: u64 = 2_400_000; // Approximate, should use calibrated value
    const TIMEOUT_MS: u64 = 100; // 100ms timeout
    let timeout_tsc = TIMEOUT_MS * TSC_PER_MS;

    // Spin until all acknowledgments received or timeout
    loop {
        if request.is_complete(expected_count) {
            return true;
        }

        // Check timeout
        let now = unsafe { core::arch::x86_64::_rdtsc() };
        if now - start >= timeout_tsc {
            return false;
        }

        // Yield to reduce bus contention
        core::hint::spin_loop();
    }
}

/// Handle TLB shootdown IPI
///
/// This function is called from the TLB flush IPI handler. It:
/// 1. Reads the current shootdown request
/// 2. Flushes the appropriate TLB entries
/// 3. Acknowledges the shootdown
///
/// # Safety
/// This function is called from an interrupt handler and must not block.
pub unsafe fn handle_tlb_shootdown_ipi() {
    // Access the current shootdown request without moving it
    let shootdown = TLB_SHOOTDOWN.lock();

    if let Some(ref req) = *shootdown {
        // Copy the values we need before releasing the lock
        let start_addr = req.start_addr;
        let end_addr = req.end_addr;

        // Acknowledge first
        req.acknowledge();

        // Release lock before flushing (flushing is slow)
        drop(shootdown);

        // Flush TLB
        if end_addr == 0 {
            // Full flush
            flush_all();
        } else {
            // Calculate page count
            const PAGE_SIZE: usize = 4096;
            let page_count = (end_addr - start_addr) / PAGE_SIZE;
            flush_range(start_addr, page_count);
        }

        // Update statistics
        percpu_current().inc_tlb_shootdowns();
    }
}

/// Mark that the current CPU has accessed a process
///
/// This should be called during context switches to track which CPUs
/// have accessed which processes. This information is used to optimize
/// TLB shootdowns.
///
/// # Arguments
/// * `pid` - Process ID that was accessed
pub fn mark_process_accessed(pid: ProcessId) {
    let cpu_id = percpu_current().id;
    if cpu_id < MAX_CPUS {
        CPU_PROCESS_MAP[cpu_id].mark_accessed(pid);
    }
}

/// Clear process tracking when a process exits
///
/// This should be called when a process terminates to clean up
/// the CPU-to-process tracking state.
///
/// # Arguments
/// * `pid` - Process ID that is exiting
pub fn clear_process_tracking(pid: ProcessId) {
    for cpu_id in 0..MAX_CPUS {
        CPU_PROCESS_MAP[cpu_id].clear_process(pid);
    }
}

/// Get a bitmask of CPUs that have accessed a process
///
/// Returns a bitmask where bit N is set if CPU N has accessed the process.
///
/// # Arguments
/// * `pid` - Process ID to check
///
/// # Returns
/// Bitmask of CPUs that have accessed this process
fn get_cpus_for_process(pid: ProcessId) -> u64 {
    let mut mask = 0u64;
    let cpu_count = get_cpu_count().min(64);

    for cpu_id in 0..cpu_count {
        if CPU_PROCESS_MAP[cpu_id].has_accessed(pid) {
            mask |= 1u64 << cpu_id;
        }
    }

    mask
}

/// Perform TLB shootdown for page table modifications
///
/// This is the main entry point for TLB shootdowns. It:
/// 1. Flushes the TLB on the current CPU
/// 2. Determines which other CPUs need to flush (based on process tracking)
/// 3. Sends IPIs to those CPUs
/// 4. Waits for acknowledgments
///
/// This function includes batch optimization: if called multiple times within 1ms,
/// the requests are coalesced into a single IPI to reduce overhead.
///
/// # Arguments
/// * `start_addr` - Starting virtual address to flush
/// * `end_addr` - Ending virtual address to flush (exclusive)
///
/// # Returns
/// `true` if shootdown completed successfully, `false` on timeout
///
/// # Safety
/// This function modifies TLB state which affects address translation.
/// The caller must ensure that the page table modifications are complete
/// before calling this function.
///
/// # Example
/// ```rust,ignore
/// // Unmap a page from the page table
/// page_table.unmap(vaddr);
///
/// // Perform TLB shootdown
/// tlb_shootdown(vaddr, vaddr + 4096);
///
/// // Now it's safe to free the physical page
/// pmm_free(phys_addr);
/// ```
pub unsafe fn tlb_shootdown(start_addr: usize, end_addr: usize) -> bool {
    let current_cpu = percpu_current().id;
    let current_tsc = core::arch::x86_64::_rdtsc();

    // Try to add to batch
    if current_cpu < MAX_CPUS {
        let batch = &BATCHED_SHOOTDOWNS[current_cpu];

        if batch.try_add(start_addr, end_addr, current_tsc) {
            // Successfully added to batch, flush will happen later
            // But we still need to flush local TLB immediately
            const PAGE_SIZE: usize = 4096;
            let page_count = (end_addr - start_addr) / PAGE_SIZE;
            if page_count == 0 || page_count > 64 {
                flush_all();
            } else {
                flush_range(start_addr, page_count);
            }
            return true;
        }

        // Batch expired or full, flush it now
        if let Some((batch_start, batch_end, batch_count)) = batch.take() {
            crate::serial_println!(
                "[TLB] Flushing batch: {} requests, range {:#x}-{:#x}",
                batch_count,
                batch_start,
                batch_end
            );

            // Perform the batched shootdown
            return tlb_shootdown_immediate(batch_start, batch_end, 0, None);
        }
    }

    // No batching, perform immediate shootdown
    tlb_shootdown_immediate(start_addr, end_addr, 0, None)
}

/// Perform TLB shootdown for a specific process
///
/// This variant allows specifying a process ID to optimize which CPUs
/// receive the shootdown IPI. Only CPUs that have accessed this process
/// will receive the IPI.
///
/// # Arguments
/// * `start_addr` - Starting virtual address to flush
/// * `end_addr` - Ending virtual address to flush (exclusive)
/// * `pid` - Process ID (for CPU filtering)
///
/// # Returns
/// `true` if shootdown completed successfully, `false` on timeout
pub unsafe fn tlb_shootdown_for_process(
    start_addr: usize,
    end_addr: usize,
    pid: ProcessId,
) -> bool {
    // Get CPUs that have accessed this process
    let cpu_mask = get_cpus_for_process(pid);

    tlb_shootdown_immediate(start_addr, end_addr, cpu_mask, Some(pid))
}

/// Perform immediate TLB shootdown without batching
///
/// This is the internal implementation that actually sends IPIs and waits
/// for acknowledgments.
///
/// # Arguments
/// * `start_addr` - Starting virtual address to flush
/// * `end_addr` - Ending virtual address to flush (exclusive)
/// * `cpu_mask` - Bitmask of CPUs to target (0 = all CPUs)
/// * `pid` - Optional process ID for logging
///
/// # Returns
/// `true` if shootdown completed successfully, `false` on timeout
unsafe fn tlb_shootdown_immediate(
    start_addr: usize,
    end_addr: usize,
    cpu_mask: u64,
    pid: Option<ProcessId>,
) -> bool {
    let seq = TLB_SHOOTDOWN_SEQ.fetch_add(1, Ordering::Relaxed);
    let current_cpu = percpu_current().id;

    // Determine which CPUs need to flush
    let target_mask = if cpu_mask == 0 {
        // All CPUs
        let cpu_count = get_cpu_count().min(64);
        (1u64 << cpu_count) - 1
    } else {
        cpu_mask
    };

    // Count how many other CPUs we need to wait for
    let mut expected_acks = 0;
    for cpu_id in 0..64 {
        if (target_mask & (1 << cpu_id)) != 0 && cpu_id != current_cpu && is_cpu_online(cpu_id) {
            expected_acks += 1;
        }
    }

    // If no other CPUs need to flush, just flush locally
    if expected_acks == 0 {
        const PAGE_SIZE: usize = 4096;
        let page_count = (end_addr - start_addr) / PAGE_SIZE;
        if page_count == 0 || page_count > 64 {
            flush_all();
        } else {
            flush_range(start_addr, page_count);
        }
        return true;
    }

    // Create shootdown request
    let request = TlbShootdownRequest::new(start_addr, end_addr);

    // Install the request globally
    {
        let mut shootdown = TLB_SHOOTDOWN.lock();
        *shootdown = Some(request);
    }

    // Send IPIs to target CPUs
    let sent_count = send_tlb_shootdown_ipis(target_mask);

    // Flush local TLB
    const PAGE_SIZE: usize = 4096;
    let page_count = (end_addr - start_addr) / PAGE_SIZE;
    if page_count == 0 || page_count > 64 {
        flush_all();
    } else {
        flush_range(start_addr, page_count);
    }

    // Wait for acknowledgments (timeout after 100ms)
    let start = core::arch::x86_64::_rdtsc();
    const TSC_PER_MS: u64 = 2_400_000;
    let timeout_tsc = 100 * TSC_PER_MS;

    let mut success = false;
    loop {
        // Check if all acknowledgments received
        let ack_count = {
            let shootdown = TLB_SHOOTDOWN.lock();
            if let Some(ref req) = *shootdown {
                req.ack_count.load(Ordering::Acquire)
            } else {
                0
            }
        };

        if ack_count >= expected_acks {
            success = true;
            break;
        }

        // Check timeout
        let now = core::arch::x86_64::_rdtsc();
        if now - start >= timeout_tsc {
            break;
        }

        core::hint::spin_loop();
    }

    // Clear the global request
    {
        let mut shootdown = TLB_SHOOTDOWN.lock();
        *shootdown = None;
    }

    // Update statistics
    percpu_current().inc_tlb_shootdowns();

    if !success {
        let pid_str = if let Some(p) = pid {
            alloc::format!(" (pid={})", p)
        } else {
            alloc::string::String::new()
        };

        crate::serial_println!(
            "[TLB] WARNING: Shootdown #{}{} timed out (sent={}, expected={})",
            seq,
            pid_str,
            sent_count,
            expected_acks
        );
    }

    success
}
