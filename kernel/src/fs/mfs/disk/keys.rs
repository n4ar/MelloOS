//! MelloFS B-tree Key and Value Types
//!
//! Defines all key and value structures for the B-tree.

use alloc::vec::Vec;
use core::cmp::Ordering;

/// Key type discriminators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum KeyType {
    DirKey = 0x01,
    InodeKey = 0x02,
    ExtentKey = 0x03,
    XattrKey = 0x04,
}

/// Directory entry key
#[derive(Debug, Clone)]
#[repr(C)]
pub struct DirKey {
    /// Key type (0x01)
    pub key_type: u8,
    /// Reserved padding
    _reserved: [u8; 7],
    /// Parent directory inode number
    pub parent_ino: u64,
    /// Hash of entry name (FNV-1a 64-bit)
    pub name_hash: u64,
    /// Length of inline name (0-64)
    pub name_len: u8,
    /// Reserved padding
    _reserved2: u8,
    /// Inline name (UTF-8, up to 64 bytes)
    pub name_inline: [u8; 64],
}

impl DirKey {
    pub const SIZE: usize = 90;

    pub fn new(parent_ino: u64, name: &str) -> Self {
        let name_bytes = name.as_bytes();
        let name_hash = fnv1a_hash(name_bytes);
        let inline_len = core::cmp::min(name_bytes.len(), 64);

        let mut name_inline = [0u8; 64];
        name_inline[..inline_len].copy_from_slice(&name_bytes[..inline_len]);

        Self {
            key_type: KeyType::DirKey as u8,
            _reserved: [0; 7],
            parent_ino,
            name_hash,
            name_len: inline_len as u8,
            _reserved2: 0,
            name_inline,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };
        bytes.to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::SIZE {
            return Err("Data too small for DirKey");
        }
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const DirKey) })
    }
}

impl PartialEq for DirKey {
    fn eq(&self, other: &Self) -> bool {
        self.parent_ino == other.parent_ino
            && self.name_hash == other.name_hash
            && self.name_inline[..self.name_len as usize]
                == other.name_inline[..other.name_len as usize]
    }
}

impl Eq for DirKey {}

impl PartialOrd for DirKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DirKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parent_ino
            .cmp(&other.parent_ino)
            .then(self.name_hash.cmp(&other.name_hash))
            .then(
                self.name_inline[..self.name_len as usize]
                    .cmp(&other.name_inline[..other.name_len as usize]),
            )
    }
}

/// Inode metadata key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct InodeKey {
    /// Key type (0x02)
    pub key_type: u8,
    /// Reserved padding
    _reserved: [u8; 7],
    /// Inode number
    pub ino: u64,
}

impl InodeKey {
    pub const SIZE: usize = 16;

    pub fn new(ino: u64) -> Self {
        Self {
            key_type: KeyType::InodeKey as u8,
            _reserved: [0; 7],
            ino,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };
        bytes.to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::SIZE {
            return Err("Data too small for InodeKey");
        }
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const InodeKey) })
    }
}

impl PartialOrd for InodeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InodeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ino.cmp(&other.ino)
    }
}

/// File extent key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ExtentKey {
    /// Key type (0x03)
    pub key_type: u8,
    /// Reserved padding
    _reserved: [u8; 7],
    /// Inode number
    pub ino: u64,
    /// Offset within file (bytes)
    pub file_offset: u64,
}

impl ExtentKey {
    pub const SIZE: usize = 24;

    pub fn new(ino: u64, file_offset: u64) -> Self {
        Self {
            key_type: KeyType::ExtentKey as u8,
            _reserved: [0; 7],
            ino,
            file_offset,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };
        bytes.to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::SIZE {
            return Err("Data too small for ExtentKey");
        }
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const ExtentKey) })
    }
}

impl PartialOrd for ExtentKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExtentKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ino
            .cmp(&other.ino)
            .then(self.file_offset.cmp(&other.file_offset))
    }
}

/// Extended attribute key
#[derive(Debug, Clone)]
#[repr(C)]
pub struct XattrKey {
    /// Key type (0x04)
    pub key_type: u8,
    /// Reserved padding
    _reserved: [u8; 7],
    /// Inode number
    pub ino: u64,
    /// Hash of attribute name (FNV-1a)
    pub name_hash: u64,
    /// Length of attribute name
    pub name_len: u8,
    /// Reserved padding
    _reserved2: u8,
    /// Attribute name (UTF-8, up to 254 bytes)
    pub name: [u8; 254],
}

impl XattrKey {
    pub const SIZE: usize = 272;

    pub fn new(ino: u64, name: &str) -> Self {
        let name_bytes = name.as_bytes();
        let name_hash = fnv1a_hash(name_bytes);
        let name_len = core::cmp::min(name_bytes.len(), 254);

        let mut name_buf = [0u8; 254];
        name_buf[..name_len].copy_from_slice(&name_bytes[..name_len]);

        Self {
            key_type: KeyType::XattrKey as u8,
            _reserved: [0; 7],
            ino,
            name_hash,
            name_len: name_len as u8,
            _reserved2: 0,
            name: name_buf,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };
        bytes.to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::SIZE {
            return Err("Data too small for XattrKey");
        }
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const XattrKey) })
    }
}

impl PartialEq for XattrKey {
    fn eq(&self, other: &Self) -> bool {
        self.ino == other.ino
            && self.name_hash == other.name_hash
            && self.name[..self.name_len as usize] == other.name[..other.name_len as usize]
    }
}

impl Eq for XattrKey {}

impl PartialOrd for XattrKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for XattrKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ino
            .cmp(&other.ino)
            .then(self.name_hash.cmp(&other.name_hash))
            .then(self.name[..self.name_len as usize].cmp(&other.name[..other.name_len as usize]))
    }
}

// ============================================================================
// Value Types
// ============================================================================

/// File type constants (POSIX)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    Fifo = 0x01,
    Chr = 0x02,
    Dir = 0x04,
    Blk = 0x06,
    Reg = 0x08,
    Lnk = 0x0A,
    Sock = 0x0C,
}

/// Directory entry value
#[derive(Debug, Clone)]
pub struct DirVal {
    /// Child inode number
    pub child_ino: u64,
    /// File type
    pub file_type: u8,
    /// Full name length (if > 64 bytes)
    pub name_len: u8,
    /// Full name (if > 64 bytes, variable length)
    pub name_overflow: Vec<u8>,
}

impl DirVal {
    pub const MIN_SIZE: usize = 12;

    pub fn new(child_ino: u64, file_type: FileType, name: Option<&str>) -> Self {
        let (name_len, name_overflow) = if let Some(n) = name {
            let bytes = n.as_bytes();
            if bytes.len() > 64 {
                (bytes.len() as u8, bytes.to_vec())
            } else {
                (0, Vec::new())
            }
        } else {
            (0, Vec::new())
        };

        Self {
            child_ino,
            file_type: file_type as u8,
            name_len,
            name_overflow,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_SIZE + self.name_overflow.len());
        bytes.extend_from_slice(&self.child_ino.to_le_bytes());
        bytes.push(self.file_type);
        bytes.push(self.name_len);
        bytes.extend_from_slice(&[0u8; 2]); // Reserved padding
        bytes.extend_from_slice(&self.name_overflow);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::MIN_SIZE {
            return Err("Data too small for DirVal");
        }

        let child_ino = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        let file_type = data[8];
        let name_len = data[9];

        let name_overflow = if name_len > 0 {
            data[Self::MIN_SIZE..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            child_ino,
            file_type,
            name_len,
            name_overflow,
        })
    }
}

/// Inode flags
pub const INODE_FLAG_IMMUTABLE: u32 = 1 << 0;
pub const INODE_FLAG_APPEND_ONLY: u32 = 1 << 1;
pub const INODE_FLAG_NODUMP: u32 = 1 << 2;
pub const INODE_FLAG_COMPRESSED: u32 = 1 << 3;

/// Inode metadata value
#[derive(Debug, Clone)]
pub struct InodeVal {
    /// File mode and type (POSIX)
    pub mode: u16,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Hard link count
    pub nlink: u32,
    /// File size in bytes
    pub size: u64,
    /// Access time (ns since epoch)
    pub atime_ns: u64,
    /// Modification time (ns since epoch)
    pub mtime_ns: u64,
    /// Status change time (ns since epoch)
    pub ctime_ns: u64,
    /// Creation time (ns since epoch)
    pub crtime_ns: u64,
    /// Inode flags
    pub flags: u32,
    /// Device ID (for special files)
    pub rdev: u64,
    /// Inline data (up to 4096 bytes)
    pub inline_data: Vec<u8>,
}

impl InodeVal {
    pub const MIN_SIZE: usize = 80;
    pub const MAX_INLINE_SIZE: usize = 4096;

    pub fn new(mode: u16, uid: u32, gid: u32) -> Self {
        Self {
            mode,
            uid,
            gid,
            nlink: 1,
            size: 0,
            atime_ns: 0,
            mtime_ns: 0,
            ctime_ns: 0,
            crtime_ns: 0,
            flags: 0,
            rdev: 0,
            inline_data: Vec::new(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let inline_len = core::cmp::min(self.inline_data.len(), Self::MAX_INLINE_SIZE);
        let mut bytes = Vec::with_capacity(Self::MIN_SIZE + inline_len);

        bytes.extend_from_slice(&self.mode.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 2]); // Reserved
        bytes.extend_from_slice(&self.uid.to_le_bytes());
        bytes.extend_from_slice(&self.gid.to_le_bytes());
        bytes.extend_from_slice(&self.nlink.to_le_bytes());
        bytes.extend_from_slice(&self.size.to_le_bytes());
        bytes.extend_from_slice(&self.atime_ns.to_le_bytes());
        bytes.extend_from_slice(&self.mtime_ns.to_le_bytes());
        bytes.extend_from_slice(&self.ctime_ns.to_le_bytes());
        bytes.extend_from_slice(&self.crtime_ns.to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&(inline_len as u16).to_le_bytes());
        bytes.extend_from_slice(&[0u8; 2]); // Reserved
        bytes.extend_from_slice(&self.rdev.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]); // Reserved
        bytes.extend_from_slice(&self.inline_data[..inline_len]);

        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::MIN_SIZE {
            return Err("Data too small for InodeVal");
        }

        let mode = u16::from_le_bytes([data[0], data[1]]);
        let uid = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let gid = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let nlink = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let size = u64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        let atime_ns = u64::from_le_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);
        let mtime_ns = u64::from_le_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);
        let ctime_ns = u64::from_le_bytes([
            data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
        ]);
        let crtime_ns = u64::from_le_bytes([
            data[48], data[49], data[50], data[51], data[52], data[53], data[54], data[55],
        ]);
        let flags = u32::from_le_bytes([data[56], data[57], data[58], data[59]]);
        let inline_len = u16::from_le_bytes([data[60], data[61]]) as usize;
        let rdev = u64::from_le_bytes([
            data[64], data[65], data[66], data[67], data[68], data[69], data[70], data[71],
        ]);

        let inline_data = if inline_len > 0 && data.len() >= Self::MIN_SIZE + inline_len {
            data[Self::MIN_SIZE..Self::MIN_SIZE + inline_len].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            mode,
            uid,
            gid,
            nlink,
            size,
            atime_ns,
            mtime_ns,
            ctime_ns,
            crtime_ns,
            flags,
            rdev,
            inline_data,
        })
    }
}

/// Extent flags
pub const EXTENT_FLAG_COMPRESSED: u16 = 1 << 0;
pub const EXTENT_FLAG_CHECKSUMMED: u16 = 1 << 1;

/// File extent value
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExtentVal {
    /// Physical block address
    pub phys_lba: u64,
    /// Length in blocks
    pub length: u32,
    /// Extent flags
    pub flags: u16,
    /// Reserved padding
    _reserved: u16,
    /// Data checksum (if CHECKSUMMED flag set)
    pub checksum: u64,
}

impl ExtentVal {
    pub const SIZE: usize = 24;

    pub fn new(phys_lba: u64, length: u32) -> Self {
        Self {
            phys_lba,
            length,
            flags: 0,
            _reserved: 0,
            checksum: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, Self::SIZE) };
        bytes.to_vec()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::SIZE {
            return Err("Data too small for ExtentVal");
        }
        Ok(unsafe { core::ptr::read(data.as_ptr() as *const ExtentVal) })
    }
}

/// Extended attribute value
#[derive(Debug, Clone)]
pub struct XattrVal {
    /// Value length in bytes
    pub length: u32,
    /// Attribute value (up to 64 KiB)
    pub data: Vec<u8>,
}

impl XattrVal {
    pub const MIN_SIZE: usize = 8;
    pub const MAX_SIZE: usize = 65536;

    pub fn new(data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() > Self::MAX_SIZE {
            return Err("Attribute value too large");
        }

        Ok(Self {
            length: data.len() as u32,
            data,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::MIN_SIZE + self.data.len());
        bytes.extend_from_slice(&self.length.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]); // Reserved
        bytes.extend_from_slice(&self.data);
        bytes
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < Self::MIN_SIZE {
            return Err("Data too small for XattrVal");
        }

        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < Self::MIN_SIZE + length {
            return Err("Truncated attribute value");
        }

        let value_data = data[Self::MIN_SIZE..Self::MIN_SIZE + length].to_vec();

        Ok(Self {
            length: length as u32,
            data: value_data,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// FNV-1a hash function (64-bit)
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_key() {
        let key = DirKey::new(1, "test.txt");
        assert_eq!(key.parent_ino, 1);
        assert_eq!(key.name_len, 8);

        let bytes = key.to_bytes();
        let key2 = DirKey::from_bytes(&bytes).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    fn test_inode_key() {
        let key = InodeKey::new(42);
        assert_eq!(key.ino, 42);

        let bytes = key.to_bytes();
        let key2 = InodeKey::from_bytes(&bytes).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    fn test_extent_key() {
        let key = ExtentKey::new(42, 4096);
        assert_eq!(key.ino, 42);
        assert_eq!(key.file_offset, 4096);

        let bytes = key.to_bytes();
        let key2 = ExtentKey::from_bytes(&bytes).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    fn test_inode_val() {
        let val = InodeVal::new(0o644, 1000, 1000);
        assert_eq!(val.mode, 0o644);
        assert_eq!(val.uid, 1000);
        assert_eq!(val.gid, 1000);

        let bytes = val.to_bytes();
        let val2 = InodeVal::from_bytes(&bytes).unwrap();
        assert_eq!(val.mode, val2.mode);
        assert_eq!(val.uid, val2.uid);
    }
}
