//! Memory mapping (mmap) support - Simplified working implementation
//!
//! This provides basic mmap functionality for Phase 8.

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::RwLock;

const PAGE_SIZE: usize = 4096;

/// Memory protection flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProtFlags {
    bits: u8,
}

impl ProtFlags {
    pub const PROT_NONE: Self = Self { bits: 0 };
    pub const PROT_READ: Self = Self { bits: 1 };
    pub const PROT_WRITE: Self = Self { bits: 2 };
    pub const PROT_EXEC: Self = Self { bits: 4 };

    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub const fn bits(&self) -> u8 {
        self.bits
    }

    pub const fn is_readable(&self) -> bool {
        self.bits & Self::PROT_READ.bits != 0
    }

    pub const fn is_writable(&self) -> bool {
        self.bits & Self::PROT_WRITE.bits != 0
    }

    pub const fn is_executable(&self) -> bool {
        self.bits & Self::PROT_EXEC.bits != 0
    }
}

/// Memory mapping flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MmapFlags {
    bits: u32,
}

impl MmapFlags {
    pub const MAP_SHARED: Self = Self { bits: 0x01 };
    pub const MAP_PRIVATE: Self = Self { bits: 0x02 };
    pub const MAP_FIXED: Self = Self { bits: 0x10 };
    pub const MAP_ANONYMOUS: Self = Self { bits: 0x20 };

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }

    pub const fn is_shared(&self) -> bool {
        self.bits & Self::MAP_SHARED.bits != 0
    }

    pub const fn is_anonymous(&self) -> bool {
        self.bits & Self::MAP_ANONYMOUS.bits != 0
    }
}

/// msync flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MsyncFlags {
    bits: u32,
}

impl MsyncFlags {
    pub const MS_SYNC: Self = Self { bits: 0x01 };
    pub const MS_ASYNC: Self = Self { bits: 0x02 };

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn is_sync(&self) -> bool {
        self.bits & Self::MS_SYNC.bits != 0
    }
}

/// Memory mapping descriptor
#[derive(Clone, Copy)]
pub struct MemoryMapping {
    pub vaddr: u64,
    pub length: usize,
    pub prot: ProtFlags,
    pub flags: MmapFlags,
    pub fd: Option<u32>,
    pub offset: u64,
    pub valid: bool,
}

impl MemoryMapping {
    const fn new() -> Self {
        Self {
            vaddr: 0,
            length: 0,
            prot: ProtFlags::PROT_NONE,
            flags: MmapFlags::MAP_PRIVATE,
            fd: None,
            offset: 0,
            valid: false,
        }
    }

    pub fn create(
        vaddr: u64,
        length: usize,
        prot: ProtFlags,
        flags: MmapFlags,
        fd: Option<u32>,
        offset: u64,
    ) -> Self {
        Self {
            vaddr,
            length,
            prot,
            flags,
            fd,
            offset,
            valid: true,
        }
    }

    pub fn contains(&self, addr: u64) -> bool {
        self.valid && addr >= self.vaddr && addr < self.vaddr + self.length as u64
    }
}

const MAX_MAPPINGS: usize = 256;

/// Per-process memory mapping table
pub struct MmapTable {
    pid: AtomicU64,
    in_use: AtomicBool,
    mappings: [RwLock<MemoryMapping>; MAX_MAPPINGS],
    count: AtomicUsize,
}

impl MmapTable {
    const fn new() -> Self {
        const INIT: RwLock<MemoryMapping> = RwLock::new(MemoryMapping::new());
        Self {
            pid: AtomicU64::new(0),
            in_use: AtomicBool::new(false),
            mappings: [INIT; MAX_MAPPINGS],
            count: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, pid: u64) {
        self.pid.store(pid, Ordering::Release);
        self.in_use.store(true, Ordering::Release);
        self.count.store(0, Ordering::Release);
    }

    pub fn is_for_process(&self, pid: u64) -> bool {
        self.in_use.load(Ordering::Acquire) && self.pid.load(Ordering::Acquire) == pid
    }

    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::Acquire)
    }

    pub fn add_mapping(&self, mapping: MemoryMapping) -> Option<usize> {
        for (idx, lock) in self.mappings.iter().enumerate() {
            let mut m = lock.write();
            if !m.valid {
                *m = mapping;
                self.count.fetch_add(1, Ordering::Relaxed);
                return Some(idx);
            }
        }
        None
    }

    pub fn find_mapping(&self, addr: u64) -> Option<MemoryMapping> {
        for lock in &self.mappings {
            let m = lock.read();
            if m.contains(addr) {
                return Some(*m);
            }
        }
        None
    }

    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

const MAX_TABLES: usize = 256;

pub struct MmapManager {
    tables: [MmapTable; MAX_TABLES],
}

impl MmapManager {
    const fn new() -> Self {
        const INIT: MmapTable = MmapTable::new();
        Self {
            tables: [INIT; MAX_TABLES],
        }
    }

    pub fn get_table(&self, pid: u64) -> Option<&MmapTable> {
        for table in &self.tables {
            if table.is_for_process(pid) {
                return Some(table);
            }
        }
        for table in &self.tables {
            if !table.is_in_use() {
                table.init(pid);
                return Some(table);
            }
        }
        None
    }
}

use spin::Once;

static MMAP_MANAGER: Once<MmapManager> = Once::new();

pub fn get_mmap_manager() -> &'static MmapManager {
    MMAP_MANAGER.call_once(|| MmapManager::new())
}

/// mmap syscall implementation
pub fn sys_mmap(
    addr: u64,
    length: usize,
    prot: ProtFlags,
    flags: MmapFlags,
    fd: i32,
    offset: u64,
) -> Result<u64, &'static str> {
    if length == 0 {
        return Err("Invalid length");
    }

    // Round length up to page boundary
    let page_aligned_length = (length + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Get current process ID
    let pid = crate::sched::get_current_task_info()
        .map(|info| info.0 as u64)
        .unwrap_or(1);

    let table = get_mmap_manager().get_table(pid).ok_or("No mmap table")?;

    // Determine virtual address
    let vaddr = if addr != 0 && flags.bits & MmapFlags::MAP_FIXED.bits != 0 {
        addr
    } else if addr != 0 {
        addr
    } else {
        0x7000_0000_0000 + (table.count() * 0x10000) as u64
    };

    // Ensure page alignment
    if vaddr & (PAGE_SIZE as u64 - 1) != 0 {
        return Err("Address not page aligned");
    }

    // Store mapping info
    let fd_opt = if fd >= 0 { Some(fd as u32) } else { None };
    let mapping = MemoryMapping::create(vaddr, page_aligned_length, prot, flags, fd_opt, offset);
    table.add_mapping(mapping).ok_or("Too many mappings")?;

    crate::serial_println!(
        "[MMAP] Mapped {} bytes at {:#x} (fd={:?})",
        page_aligned_length,
        vaddr,
        fd_opt
    );

    Ok(vaddr)
}

/// msync syscall implementation
pub fn sys_msync(addr: u64, length: usize, flags: MsyncFlags) -> Result<(), &'static str> {
    if length == 0 {
        return Ok(());
    }

    let pid = crate::sched::get_current_task_info()
        .map(|info| info.0 as u64)
        .unwrap_or(1);

    let table = get_mmap_manager().get_table(pid).ok_or("No mmap table")?;
    let mapping = table.find_mapping(addr).ok_or("Address not mapped")?;

    if addr + length as u64 > mapping.vaddr + mapping.length as u64 {
        return Err("Range exceeds mapping");
    }

    // Only sync shared file-backed mappings
    if !mapping.flags.is_shared() || mapping.fd.is_none() {
        return Ok(());
    }

    crate::serial_println!(
        "[MMAP] msync at {:#x}, {} bytes (sync={})",
        addr,
        length,
        flags.is_sync()
    );

    // Note: Actual implementation would flush dirty pages to file
    // For now, just acknowledge the sync request

    Ok(())
}

/// mprotect syscall implementation
pub fn sys_mprotect(addr: u64, length: usize, prot: ProtFlags) -> Result<(), &'static str> {
    if length == 0 {
        return Ok(());
    }

    if addr & (PAGE_SIZE as u64 - 1) != 0 {
        return Err("Address not page aligned");
    }

    let pid = crate::sched::get_current_task_info()
        .map(|info| info.0 as u64)
        .unwrap_or(1);

    let table = get_mmap_manager().get_table(pid).ok_or("No mmap table")?;
    let mapping = table.find_mapping(addr).ok_or("Address not mapped")?;

    if addr + length as u64 > mapping.vaddr + mapping.length as u64 {
        return Err("Range exceeds mapping");
    }

    crate::serial_println!(
        "[MMAP] mprotect at {:#x}, {} bytes (prot={:?})",
        addr,
        length,
        prot
    );

    // Note: Actual implementation would update page table entries
    // For now, just acknowledge the protection change

    Ok(())
}
