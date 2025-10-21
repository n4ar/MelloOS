//! TLB Shootdown Implementation
//!
//! This module implements TLB (Translation Lookaside Buffer) shootdown for
//! multicore systems. When page tables are modified, all CPUs that might have
//! cached translations must flush their TLBs to ensure memory consistency.

use crate::arch::x86_64::apic::ipi::{send_ipi, TLB_FLUSH_IPI_VECTOR};
use crate::arch::x86_64::smp::percpu::{percpu_current, percpu_for};
use crate::arch::x86_64::smp::{get_cpu_count, is_cpu_online};
use crate::sync::SpinLock;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Maximum number of CPUs that can be targeted in a single shootdown
const MAX_SHOOTDOWN_CPUS: usize = 64;

/// TLB shootdown request structure
///
/// Contains information about a TLB flush request that needs to be
/// propagated to other CPU cores.
#[derive(Debug, Clone, Copy)]
pub struct TlbShootdownRequest {
    /// Virtual address to flush (or 0 for full TLB flush)
    pub vaddr: usize,
    /// Number of pages to flush (or 0 for full TLB flush)
    pub page_count: usize,
    /// Bitmask of CPUs that need to acknowledge
    pub cpu_mask: u64,
    /// Number of CPUs that have acknowledged
    pub ack_count: AtomicUsize,
}

impl TlbShootdownRequest {
    /// Create a new TLB shootdown request
    pub fn new(vaddr: usize, page_count: usize, cpu_mask: u64) -> Self {
        Self {
            vaddr,
            page_count,
            cpu_mask,
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
        "mov cr3, {}",
        out(reg) cr3,
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
/// * `timeout_ms` - Maximum time to wait in milliseconds
///
/// # Returns
/// `true` if all acknowledgments received, `false` on timeout
fn wait_for_acks(request: &TlbShootdownRequest, expected_count: usize, timeout_ms: u64) -> bool {
    // Get current timestamp
    let start = unsafe { core::arch::x86_64::_rdtsc() };
    const TSC_PER_MS: u64 = 2_400_000; // Approximate, should use calibrated value
    let timeout_tsc = timeout_ms * TSC_PER_MS;

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

/// Perform TLB shootdown for page table modifications
///
/// This function coordinates a TLB flush across multiple CPU cores. It:
/// 1. Flushes the TLB on the current CPU
/// 2. Sends IPIs to other CPUs that might have cached translations
/// 3. Waits for acknowledgments from all target CPUs
/// 4. Returns when all CPUs have flushed their TLBs
///
/// # Arguments
/// * `vaddr` - Virtual address to flush (or 0 for full flush)
/// * `page_count` - Number of pages to flush (or 0 for full flush)
/// * `cpu_mask` - Bitmask of CPUs that need to flush (0 = all CPUs)
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
/// // Perform TLB shootdown on all CPUs
/// tlb_shootdown(vaddr, 1, 0);
///
/// // Now it's safe to free the physical page
/// pmm_free(phys_addr);
/// ```
pub unsafe fn tlb_shootdown(vaddr: usize, page_count: usize, cpu_mask: u64) -> bool {
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
        if page_count == 0 {
            flush_all();
        } else {
            flush_range(vaddr, page_count);
        }
        return true;
    }

    // Create shootdown request
    let request = TlbShootdownRequest::new(vaddr, page_count, target_mask);

    // Install the request globally
    {
        let mut shootdown = TLB_SHOOTDOWN.lock();
        *shootdown = Some(request);
    }

    // Send IPIs to target CPUs
    let sent_count = send_tlb_shootdown_ipis(target_mask);

    // Flush local TLB
    if page_count == 0 {
        flush_all();
    } else {
        flush_range(vaddr, page_count);
    }

    // Wait for acknowledgments (timeout after 100ms)
    let success = wait_for_acks(&request, expected_acks, 100);

    // Clear the global request
    {
        let mut shootdown = TLB_SHOOTDOWN.lock();
        *shootdown = None;
    }

    // Update statistics
    percpu_current().inc_tlb_shootdowns();

    if !success {
        crate::serial_println!(
            "[TLB] WARNING: Shootdown #{} timed out (sent={}, expected={})",
            seq,
            sent_count,
            expected_acks
        );
    }

    success
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
    // Get the current shootdown request
    let request = {
        let shootdown = TLB_SHOOTDOWN.lock();
        *shootdown
    };

    if let Some(req) = request {
        // Check if this CPU is in the target mask
        let current_cpu = percpu_current().id;
        if (req.cpu_mask & (1 << current_cpu)) != 0 {
            // Flush TLB
            if req.page_count == 0 {
                flush_all();
            } else {
                flush_range(req.vaddr, req.page_count);
            }

            // Acknowledge
            req.acknowledge();

            // Update statistics
            percpu_current().inc_tlb_shootdowns();
        }
    }
}
