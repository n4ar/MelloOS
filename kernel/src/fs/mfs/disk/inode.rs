//! MelloFS Disk Inode Implementation
//!
//! Implements the Inode trait for persistent MelloFS inodes stored on disk.

use crate::fs::block_dev::BlockDevice;
use crate::fs::vfs::inode::{DirCookie, DirEnt, FileMode, Inode, SetAttr, Stat};
use crate::fs::vfs::superblock::FsError;
use crate::sync::SpinLock;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

/// On-disk inode structure (256 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DiskInodeData {
    /// File mode and type
    pub mode: u16,
    /// User ID
    pub uid: u32,
    /// Group ID  
    pub gid: u32,
    /// File size in bytes
    pub size: u64,
    /// Access time (Unix timestamp)
    pub atime: u64,
    /// Modification time (Unix timestamp)
    pub mtime: u64,
    /// Change time (Unix timestamp)
    pub ctime: u64,
    /// Number of hard links
    pub nlink: u32,
    /// Number of 512-byte blocks allocated
    pub blocks: u64,
    /// Direct block pointers (12 blocks)
    pub direct: [u64; 12],
    /// Indirect block pointer
    pub indirect: u64,
    /// Double indirect block pointer
    pub double_indirect: u64,
    /// Triple indirect block pointer
    pub triple_indirect: u64,
    /// Reserved for future use
    pub reserved: [u64; 8],
}

impl DiskInodeData {
    pub const SIZE: usize = 256;

    pub fn new(mode: FileMode, uid: u32, gid: u32) -> Self {
        let now = super::super_::current_time_ns() / 1_000_000_000; // Convert to seconds
        
        Self {
            mode: mode.bits(),
            uid,
            gid,
            size: 0,
            atime: now,
            mtime: now,
            ctime: now,
            nlink: if mode.is_dir() { 2 } else { 1 }, // Directories start with 2 links (. and ..)
            blocks: 0,
            direct: [0; 12],
            indirect: 0,
            double_indirect: 0,
            triple_indirect: 0,
            reserved: [0; 8],
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, FsError> {
        if data.len() < Self::SIZE {
            return Err(FsError::InvalidArgument);
        }

        let inode_data = unsafe {
            core::ptr::read(data.as_ptr() as *const DiskInodeData)
        };

        Ok(inode_data)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const DiskInodeData as *const u8,
                Self::SIZE,
            )
        };
        bytes.to_vec()
    }
}

/// MelloFS Disk Inode
pub struct MfsDiskInode {
    /// Inode number
    inode_id: u64,
    /// On-disk inode data
    data: SpinLock<DiskInodeData>,
    /// Block device for I/O
    device: Arc<dyn BlockDevice>,
    /// Cached directory entries (for directories)
    dir_entries: SpinLock<Option<Vec<DirEnt>>>,
}

impl MfsDiskInode {
    /// Create a new directory inode
    pub fn new_directory(
        inode_id: u64,
        mode: FileMode,
        uid: u32,
        gid: u32,
        device: Arc<dyn BlockDevice>,
    ) -> Result<Self, FsError> {
        let data = DiskInodeData::new(mode, uid, gid);
        
        Ok(Self {
            inode_id,
            data: SpinLock::new(data),
            device,
            dir_entries: SpinLock::new(None),
        })
    }

    /// Create inode from disk data
    pub fn from_disk_data(
        inode_id: u64,
        disk_data: &[u8],
        device: Arc<dyn BlockDevice>,
    ) -> Result<Self, FsError> {
        let data = DiskInodeData::from_bytes(disk_data)?;
        
        Ok(Self {
            inode_id,
            data: SpinLock::new(data),
            device,
            dir_entries: SpinLock::new(None),
        })
    }

    /// Initialize root directory with "." and ".." entries
    pub fn init_root_directory(&self) -> Result<(), FsError> {
        let mut entries = Vec::new();
        
        // Add "." entry (self)
        entries.push(DirEnt {
            ino: self.inode_id,
            name: String::from("."),
        });
        
        // Add ".." entry (parent, same as self for root)
        entries.push(DirEnt {
            ino: self.inode_id,
            name: String::from(".."),
        });

        // Cache the entries
        let mut dir_entries = self.dir_entries.lock();
        *dir_entries = Some(entries);

        // Write directory entries to disk
        self.write_directory_entries_to_disk()?;

        crate::serial_println!("[MFS_DISK] Root directory initialized with . and .. entries");

        Ok(())
    }

    /// Write directory entries to disk
    fn write_directory_entries_to_disk(&self) -> Result<(), FsError> {
        let dir_entries = self.dir_entries.lock();
        let entries = dir_entries.as_ref().ok_or(FsError::InvalidArgument)?;

        // Serialize directory entries
        let mut data = Vec::new();
        for entry in entries {
            // Entry format: [inode_id: 8 bytes][name_len: 2 bytes][name: variable]
            data.extend_from_slice(&entry.ino.to_le_bytes());
            data.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            data.extend_from_slice(entry.name.as_bytes());
        }

        // Pad to block boundary
        let block_size = 4096;
        let padded_size = (data.len() + block_size - 1) & !(block_size - 1);
        data.resize(padded_size, 0);

        // Allocate block for directory data
        let block_lba = self.allocate_data_block()?;
        
        // Write to disk
        self.device
            .write_block(block_lba, &data)
            .map_err(|_| FsError::IoError)?;

        // Update inode data
        let mut inode_data = self.data.lock();
        inode_data.direct[0] = block_lba;
        inode_data.size = data.len() as u64;
        inode_data.blocks = (padded_size / 512) as u64;

        // Write updated inode to disk
        self.write_inode_to_disk(&*inode_data)?;

        Ok(())
    }

    /// Allocate a data block
    fn allocate_data_block(&self) -> Result<u64, FsError> {
        // Simple allocation: use block numbers starting from 1000
        // In a complete implementation, this would use the allocator B-tree
        static mut NEXT_BLOCK: u64 = 1000;
        
        unsafe {
            let block = NEXT_BLOCK;
            NEXT_BLOCK += 1;
            Ok(block)
        }
    }

    /// Write inode data to disk
    fn write_inode_to_disk(&self, inode_data: &DiskInodeData) -> Result<(), FsError> {
        // Calculate inode location on disk
        let inode_table_start = 32; // LBA 32 (after superblock)
        let inode_size = 256;
        let inodes_per_block = 4096 / inode_size; // 16 inodes per block
        
        let inode_block = inode_table_start + ((self.inode_id - 1) / inodes_per_block as u64);
        let inode_offset = ((self.inode_id - 1) % inodes_per_block as u64) * inode_size as u64;

        // Read the block containing the inode
        let mut buffer = alloc::vec![0u8; 4096];
        self.device
            .read_block(inode_block, &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Update inode data in the buffer
        let inode_bytes = inode_data.to_bytes();
        buffer[inode_offset as usize..(inode_offset as usize + inode_size)]
            .copy_from_slice(&inode_bytes);

        // Write the block back to disk
        self.device
            .write_block(inode_block, &buffer)
            .map_err(|_| FsError::IoError)?;

        Ok(())
    }

    /// Load directory entries from disk
    fn load_directory_entries(&self) -> Result<Vec<DirEnt>, FsError> {
        let inode_data = self.data.lock();
        
        if inode_data.direct[0] == 0 {
            // No data blocks allocated
            return Ok(Vec::new());
        }

        // Read directory data from first direct block
        let mut buffer = alloc::vec![0u8; 4096];
        self.device
            .read_block(inode_data.direct[0], &mut buffer)
            .map_err(|_| FsError::IoError)?;

        // Parse directory entries
        let mut entries = Vec::new();
        let mut offset = 0;
        
        while offset + 10 <= buffer.len() { // Minimum entry size: 8 + 2 = 10 bytes
            // Read inode ID
            let ino = u64::from_le_bytes([
                buffer[offset], buffer[offset + 1], buffer[offset + 2], buffer[offset + 3],
                buffer[offset + 4], buffer[offset + 5], buffer[offset + 6], buffer[offset + 7],
            ]);
            offset += 8;

            if ino == 0 {
                break; // End of entries
            }

            // Read name length
            let name_len = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]) as usize;
            offset += 2;

            if offset + name_len > buffer.len() {
                break; // Invalid entry
            }

            // Read name
            let name_bytes = &buffer[offset..offset + name_len];
            let name = String::from_utf8_lossy(name_bytes).to_string();
            offset += name_len;

            entries.push(DirEnt { ino, name });
        }

        Ok(entries)
    }
}

impl Inode for MfsDiskInode {
    fn stat(&self) -> Result<Stat, FsError> {
        let data = self.data.lock();
        
        Ok(Stat {
            st_dev: 0, // Device ID (placeholder)
            st_ino: self.inode_id,
            st_mode: data.mode as u32,
            st_nlink: data.nlink as u64,
            st_uid: data.uid,
            st_gid: data.gid,
            st_rdev: 0,
            st_size: data.size,
            st_blksize: 4096,
            st_blocks: data.blocks,
            st_atime_sec: data.atime,
            st_atime_nsec: 0,
            st_mtime_sec: data.mtime,
            st_mtime_nsec: 0,
            st_ctime_sec: data.ctime,
            st_ctime_nsec: 0,
        })
    }

    fn mode(&self) -> FileMode {
        let data = self.data.lock();
        FileMode::new(data.mode)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize, FsError> {
        let data = self.data.lock();
        
        if offset >= data.size as usize {
            return Ok(0);
        }

        // For simplicity, only support reading from first direct block
        if data.direct[0] == 0 {
            return Ok(0);
        }

        let mut block_buf = alloc::vec![0u8; 4096];
        self.device
            .read_block(data.direct[0], &mut block_buf)
            .map_err(|_| FsError::IoError)?;

        let available = (data.size as usize).saturating_sub(offset);
        let to_read = core::cmp::min(buf.len(), available);
        let to_read = core::cmp::min(to_read, block_buf.len() - offset);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&block_buf[offset..offset + to_read]);
        }

        Ok(to_read)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize, FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn truncate(&self, size: u64) -> Result<(), FsError> {
        let mut data = self.data.lock();
        data.size = size;
        data.mtime = super::super_::current_time_ns() / 1_000_000_000;
        
        // Write updated inode to disk
        self.write_inode_to_disk(&*data)?;
        
        Ok(())
    }

    fn create(&self, name: &str, mode: FileMode, uid: u32, gid: u32) -> Result<Arc<dyn Inode>, FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn mkdir(&self, name: &str, mode: FileMode, uid: u32, gid: u32) -> Result<Arc<dyn Inode>, FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn unlink(&self, name: &str) -> Result<(), FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn rmdir(&self, name: &str) -> Result<(), FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, FsError> {
        // Load directory entries if not cached
        let mut dir_entries = self.dir_entries.lock();
        if dir_entries.is_none() {
            let entries = self.load_directory_entries()?;
            *dir_entries = Some(entries);
        }

        // Search for the entry
        if let Some(ref entries) = *dir_entries {
            for entry in entries {
                if entry.name == name {
                    // Load the inode (simplified - would need superblock reference)
                    // For now, return self for "." and ".."
                    if name == "." || name == ".." {
                        return Ok(Arc::new(Self::from_disk_data(
                            entry.ino,
                            &DiskInodeData::new(FileMode::new(FileMode::S_IFDIR | 0o755), 0, 0).to_bytes(),
                            self.device.clone(),
                        )?) as Arc<dyn Inode>);
                    }
                }
            }
        }

        Err(FsError::NotFound)
    }

    fn readdir(&self, cookie: DirCookie) -> Result<Option<DirEnt>, FsError> {
        // Load directory entries if not cached
        let mut dir_entries = self.dir_entries.lock();
        if dir_entries.is_none() {
            let entries = self.load_directory_entries()?;
            *dir_entries = Some(entries);
        }

        if let Some(ref entries) = *dir_entries {
            let index = cookie.0 as usize;
            if index < entries.len() {
                return Ok(Some(entries[index].clone()));
            }
        }

        Ok(None)
    }

    fn link(&self, name: &str, target: Arc<dyn Inode>) -> Result<(), FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, FsError> {
        // For simplicity, not implemented in this basic version
        Err(FsError::NotSupported)
    }

    fn readlink(&self) -> Result<String, FsError> {
        Err(FsError::NotSupported)
    }

    fn setattr(&self, attr: &SetAttr) -> Result<(), FsError> {
        let mut data = self.data.lock();
        
        if let Some(mode) = attr.mode {
            data.mode = mode.bits();
        }
        if let Some(uid) = attr.uid {
            data.uid = uid;
        }
        if let Some(gid) = attr.gid {
            data.gid = gid;
        }
        if let Some(size) = attr.size {
            data.size = size;
        }
        
        data.ctime = super::super_::current_time_ns() / 1_000_000_000;
        
        // Write updated inode to disk
        self.write_inode_to_disk(&*data)?;
        
        Ok(())
    }

    fn sync(&self) -> Result<(), FsError> {
        // Flush device
        self.device.flush().map_err(|_| FsError::IoError)?;
        Ok(())
    }
}