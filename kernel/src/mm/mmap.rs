//! Memory mapping (mmap) support
//!
//! This module implements:
//! - mmap syscall for file-backed memory mappings
//! - msync syscall for synchronizing mapped regions
//! - mprotect syscall for changing protection
//! - Integration with page cache for coherence

use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use spin::RwLock;

/// Memory protection flags (compatible with POSIX)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProtFlags {
    bits: u8,
}

impl ProtFlags {
    pub const NONE: Self = Self { bits: 0 };
    pub const READ: Self = Self { bits: 1 };
    pub const WRITE: Self = Self { bits: 2 };
    pub const EXEC: Self = Self { bits: 4 };

    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub const fn bits(&self) -> u8 {
        self.bits
    }

    pub const fn is_readable(&self) -> bool {
        self.bits & Self::READ.bits != 0
    }

    pub const fn is_writable(&self) -> bool {
        self.bits & Self::WRITE.bits != 0
    }

    pub const fn is_executable(&self) -> bool {
        self.bits & Self::EXEC.bits != 0
    }
}

/// Memory mapping flags (compatible with POSIX)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MapFlags {
    bits: u32,
}

impl MapFlags {
    pub const SHARED: Self = Self { bits: 0x01 };
    pub const PRIVATE: Self = Self { bits: 0x02 };
    pub const FIXED: Self = Self { bits: 0x10 };
    pub const ANONYMOUS: Self = Self { bits: 0x20 };

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn bits(&self) -> u32 {
        self.bits
    }

    pub const fn is_shared(&self) -> bool {
        self.bits & Self::SHARED.bits != 0
    }

    pub const fn is_anonymous(&self) -> bool {
        self.bits & Self::ANONYMOUS.bits != 0
    }
}

/// msync flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MsyncFlags {
    bits: u32,
}

impl MsyncFlags {
    pub const SYNC: Self = Self { bits: 0x01 };
    pub const ASYNC: Self = Self { bits: 0x02 };

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }

    pub const fn is_sync(&self) -> bool {
        self.bits & Self::SYNC.bits != 0
    }
}

/// Memory mapping descriptor
#[derive(Clone, Copy)]
pub struct MemoryMapping {
    pub vaddr: u64,
    pub length: usize,
    pub prot: ProtFlags,
    pub flags: MapFlags,
    pub fd: Option<u32>,
    pub offset: u64,
    pub valid: bool,
}

impl MemoryMapping {
    const fn new() -> Self {
        Self {
            vaddr: 0,
            length: 0,
            prot: ProtFlags::NONE,
            flags: MapFlags::PRIVATE,
            fd: None,
            offset: 0,
            valid: false,
        }
    }

    pub fn create(
        vaddr: u64,
        length: usize,
        prot: ProtFlags,
        flags: MapFlags,
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

/// mmap syscall stub
pub fn sys_mmap(
    addr: u64,
    length: usize,
    prot: ProtFlags,
    flags: MapFlags,
    fd: i32,
    offset: u64,
) -> Result<u64, &'static str> {
    if length == 0 {
        return Err("Invalid length");
    }

    let pid = 1; // TODO: Get current PID
    let table = get_mmap_manager().get_table(pid).ok_or("No table")?;

    let vaddr = if addr != 0 {
        addr
    } else {
        0x7000_0000_0000 + (table.count() * 0x1000) as u64
    };

    let fd_opt = if fd >= 0 { Some(fd as u32) } else { None };
    let mapping = MemoryMapping::create(vaddr, length, prot, flags, fd_opt, offset);
    table.add_mapping(mapping).ok_or("Too many mappings")?;

    Ok(vaddr)
}

/// msync syscall stub
pub fn sys_msync(_addr: u64, _length: usize, _flags: MsyncFlags) -> Result<(), &'static str> {
    // TODO: Implement when page cache is integrated
    Ok(())
}

/// mprotect syscall stub
pub fn sys_mprotect(_addr: u64, _length: usize, _prot: ProtFlags) -> Result<(), &'static str> {
    // TODO: Implement page table updates
    Ok(())
}
