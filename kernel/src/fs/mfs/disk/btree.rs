//! MelloFS B-tree Node Structure
//!
//! B-tree nodes for metadata indexing.

use super::checksum::crc32c_u64;
use alloc::vec::Vec;

/// B-tree node magic: "MFN1"
pub const BTREE_NODE_MAGIC: u32 = 0x4D464E31;

/// Child pointer in internal nodes
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ChildPtr {
    /// Physical block address
    pub lba: u64,
    /// Length in blocks
    pub length: u32,
    /// Expected CRC32C checksum
    pub checksum: u64,
    /// Child node level
    pub level: u8,
    /// Reserved padding
    _reserved: [u8; 3],
}

impl ChildPtr {
    pub const SIZE: usize = 24;

    pub const fn new() -> Self {
        Self {
            lba: 0,
            length: 0,
            checksum: 0,
            level: 0,
            _reserved: [0; 3],
        }
    }
}

/// B-tree node header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BtreeNodeHeader {
    /// Node magic: 0x4D464E31 ("MFN1")
    pub magic: u32,
    /// Node level (0 = leaf, >0 = internal)
    pub level: u16,
    /// Number of keys in node
    pub nkeys: u16,
    /// Transaction group ID when created
    pub txg_id: u64,
    /// Unique node identifier
    pub node_id: u64,
    /// CRC32C of entire node
    pub checksum: u64,
    /// Parent node ID (0 for root)
    pub parent_node_id: u64,
    /// Reserved for future use
    _reserved: u64,
}

impl BtreeNodeHeader {
    pub const SIZE: usize = 48;

    pub const fn new() -> Self {
        Self {
            magic: BTREE_NODE_MAGIC,
            level: 0,
            nkeys: 0,
            txg_id: 0,
            node_id: 0,
            checksum: 0,
            parent_node_id: 0,
            _reserved: 0,
        }
    }
}

/// B-tree node (in-memory representation)
///
/// On-disk layout:
/// [Header][Key 0][Key 1]...[Key N-1][Value/Child 0][Value/Child 1]...[Padding]
#[derive(Debug, Clone)]
pub struct BtreeNode {
    /// Node header
    pub header: BtreeNodeHeader,
    /// Keys (sorted)
    pub keys: Vec<Vec<u8>>,
    /// Values (for leaf nodes) or child pointers (for internal nodes)
    pub values: Vec<Vec<u8>>,
    /// Block size for this node
    pub block_size: u32,
}

impl BtreeNode {
    /// Create a new empty B-tree node
    pub fn new(level: u16, block_size: u32, node_id: u64, txg_id: u64) -> Self {
        let mut header = BtreeNodeHeader::new();
        header.level = level;
        header.node_id = node_id;
        header.txg_id = txg_id;

        Self {
            header,
            keys: Vec::new(),
            values: Vec::new(),
            block_size,
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.header.level == 0
    }

    /// Check if node is full
    pub fn is_full(&self, max_keys: usize) -> bool {
        self.keys.len() >= max_keys
    }

    /// Check if node is underfull (for merging)
    pub fn is_underfull(&self, min_keys: usize) -> bool {
        self.keys.len() < min_keys
    }

    /// Get number of keys
    pub fn num_keys(&self) -> usize {
        self.keys.len()
    }

    /// Insert a key-value pair at the specified index
    pub fn insert_at(&mut self, index: usize, key: Vec<u8>, value: Vec<u8>) {
        self.keys.insert(index, key);
        self.values.insert(index, value);
        self.header.nkeys = self.keys.len() as u16;
    }

    /// Remove a key-value pair at the specified index
    pub fn remove_at(&mut self, index: usize) -> (Vec<u8>, Vec<u8>) {
        let key = self.keys.remove(index);
        let value = self.values.remove(index);
        self.header.nkeys = self.keys.len() as u16;
        (key, value)
    }

    /// Find the index where a key should be inserted (binary search)
    pub fn find_key_index(&self, key: &[u8]) -> Result<usize, usize> {
        self.keys.binary_search_by(|k| k.as_slice().cmp(key))
    }

    /// Serialize node to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.block_size as usize];
        let mut offset = 0;

        // Write header (will update checksum later)
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                BtreeNodeHeader::SIZE,
            )
        };
        buffer[offset..offset + BtreeNodeHeader::SIZE].copy_from_slice(header_bytes);
        offset += BtreeNodeHeader::SIZE;

        // Calculate space for keys and values
        let keys_size: usize = self.keys.iter().map(|k| k.len()).sum();
        let values_size: usize = self.values.iter().map(|v| v.len()).sum();

        // Check if data fits in block
        let required_size = BtreeNodeHeader::SIZE
            + keys_size
            + values_size
            + (self.keys.len() * 4)
            + (self.values.len() * 4); // Size prefixes
        if required_size > self.block_size as usize {
            return Err("Node data exceeds block size");
        }

        // Write keys with length prefixes
        for key in &self.keys {
            let len = key.len() as u32;
            buffer[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
            offset += 4;
            buffer[offset..offset + key.len()].copy_from_slice(key);
            offset += key.len();
        }

        // Write values with length prefixes
        for value in &self.values {
            let len = value.len() as u32;
            buffer[offset..offset + 4].copy_from_slice(&len.to_le_bytes());
            offset += 4;
            buffer[offset..offset + value.len()].copy_from_slice(value);
            offset += value.len();
        }

        // Compute checksum
        let checksum = crc32c_u64(&buffer[..self.block_size as usize]);

        // Update checksum in header
        buffer[24..32].copy_from_slice(&checksum.to_le_bytes());

        Ok(buffer)
    }

    /// Deserialize node from bytes
    pub fn deserialize(data: &[u8], block_size: u32) -> Result<Self, &'static str> {
        if data.len() < BtreeNodeHeader::SIZE {
            return Err("Data too small for header");
        }

        // Parse header
        let header = unsafe { core::ptr::read(data.as_ptr() as *const BtreeNodeHeader) };

        // Verify magic
        if header.magic != BTREE_NODE_MAGIC {
            return Err("Invalid node magic");
        }

        // Verify checksum
        let stored_checksum = header.checksum;
        let mut data_copy = data.to_vec();
        // Zero out checksum field for verification
        data_copy[24..32].fill(0);
        let computed_checksum = crc32c_u64(&data_copy);

        if stored_checksum != computed_checksum {
            // Log corruption details
            crate::log_error!(
                "MFS",
                "B-tree node checksum mismatch: node_id={}, expected={:#x}, got={:#x}",
                header.node_id,
                stored_checksum,
                computed_checksum
            );
            return Err("Checksum mismatch");
        }

        let mut offset = BtreeNodeHeader::SIZE;
        let mut keys = Vec::new();
        let mut values = Vec::new();

        // Read keys
        for _ in 0..header.nkeys {
            if offset + 4 > data.len() {
                return Err("Truncated key length");
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + len > data.len() {
                return Err("Truncated key data");
            }
            keys.push(data[offset..offset + len].to_vec());
            offset += len;
        }

        // Read values (for leaf) or child pointers (for internal)
        let num_values = if header.level == 0 {
            header.nkeys as usize
        } else {
            (header.nkeys as usize) + 1 // Internal nodes have N+1 children
        };

        for _ in 0..num_values {
            if offset + 4 > data.len() {
                return Err("Truncated value length");
            }
            let len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + len > data.len() {
                return Err("Truncated value data");
            }
            values.push(data[offset..offset + len].to_vec());
            offset += len;
        }

        Ok(Self {
            header,
            keys,
            values,
            block_size,
        })
    }

    /// Split node into two nodes (for insertion when full)
    pub fn split(&mut self, new_node_id: u64, txg_id: u64) -> (Vec<u8>, BtreeNode) {
        let mid = self.keys.len() / 2;

        // Middle key moves up to parent
        let middle_key = self.keys[mid].clone();

        // Create new node with right half
        let mut new_node = BtreeNode::new(self.header.level, self.block_size, new_node_id, txg_id);
        new_node.keys = self.keys.split_off(mid + 1);
        new_node.values = if self.is_leaf() {
            self.values.split_off(mid + 1)
        } else {
            self.values.split_off(mid + 1)
        };

        // Remove middle key from left node
        self.keys.pop();
        if self.is_leaf() {
            // For leaf nodes, keep the value in left node
        } else {
            // For internal nodes, middle value goes to new node
        }

        self.header.nkeys = self.keys.len() as u16;
        new_node.header.nkeys = new_node.keys.len() as u16;

        (middle_key, new_node)
    }

    /// Merge with sibling node (for deletion when underfull)
    pub fn merge(&mut self, separator_key: Vec<u8>, mut sibling: BtreeNode) {
        if !self.is_leaf() {
            // For internal nodes, add separator key
            self.keys.push(separator_key);
        }

        // Append sibling's keys and values
        self.keys.append(&mut sibling.keys);
        self.values.append(&mut sibling.values);

        self.header.nkeys = self.keys.len() as u16;
    }
}

/// B-tree operations (search, insert, delete)
pub struct BtreeOps {
    /// Block size
    block_size: u32,
    /// Maximum keys per node
    max_keys: usize,
    /// Minimum keys per node (for balancing)
    min_keys: usize,
}

impl BtreeOps {
    pub fn new(block_size: u32) -> Self {
        // Conservative estimate: assume average key size of 90 bytes, value size of 100 bytes
        let max_keys = ((block_size as usize - BtreeNodeHeader::SIZE) / 200).max(4);
        let min_keys = max_keys / 2;

        Self {
            block_size,
            max_keys,
            min_keys,
        }
    }

    /// Search for a key in the B-tree
    ///
    /// Returns the value if found, or None if not found.
    pub fn search(&self, root: &BtreeNode, key: &[u8]) -> Option<Vec<u8>> {
        let current = root;

        loop {
            // Binary search for key in current node
            match current.find_key_index(key) {
                Ok(index) => {
                    // Exact match found
                    if current.is_leaf() {
                        return Some(current.values[index].clone());
                    } else {
                        // In internal node, follow child pointer
                        // (This is simplified; real implementation would load child from disk)
                        return None;
                    }
                }
                Err(_index) => {
                    // Key not found in this node
                    if current.is_leaf() {
                        return None;
                    } else {
                        // Follow child pointer at index
                        // (This is simplified; real implementation would load child from disk)
                        return None;
                    }
                }
            }
        }
    }

    /// Insert a key-value pair into the B-tree
    ///
    /// Returns Ok(()) if successful, or Err if the node needs to be split.
    /// The caller is responsible for handling splits and updating parent nodes.
    pub fn insert_into_node(
        &self,
        node: &mut BtreeNode,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<(), InsertResult> {
        // Check if node is full
        if node.is_full(self.max_keys) {
            return Err(InsertResult::NodeFull);
        }

        // Find insertion point
        match node.find_key_index(&key) {
            Ok(_) => {
                // Key already exists
                Err(InsertResult::KeyExists)
            }
            Err(index) => {
                // Insert at this position
                node.insert_at(index, key, value);
                Ok(())
            }
        }
    }

    /// Delete a key from the B-tree node
    ///
    /// Returns Ok(value) if successful, or Err if the key was not found
    /// or the node becomes underfull.
    pub fn delete_from_node(
        &self,
        node: &mut BtreeNode,
        key: &[u8],
    ) -> Result<Vec<u8>, DeleteResult> {
        // Find key
        match node.find_key_index(key) {
            Ok(index) => {
                // Remove key-value pair
                let (_, value) = node.remove_at(index);

                // Check if node is underfull
                if node.is_underfull(self.min_keys) && node.header.parent_node_id != 0 {
                    return Err(DeleteResult::NodeUnderfull(value));
                }

                Ok(value)
            }
            Err(_) => Err(DeleteResult::KeyNotFound),
        }
    }

    /// Split a full node during insertion
    ///
    /// Returns the middle key and the new right node.
    pub fn split_node(
        &self,
        node: &mut BtreeNode,
        new_node_id: u64,
        txg_id: u64,
    ) -> (Vec<u8>, BtreeNode) {
        node.split(new_node_id, txg_id)
    }

    /// Merge an underfull node with its sibling
    pub fn merge_nodes(&self, left: &mut BtreeNode, separator_key: Vec<u8>, right: BtreeNode) {
        left.merge(separator_key, right);
    }
}

/// Result of insert operation
#[derive(Debug)]
pub enum InsertResult {
    /// Node is full and needs to be split
    NodeFull,
    /// Key already exists
    KeyExists,
}

/// Result of delete operation
#[derive(Debug)]
pub enum DeleteResult {
    /// Key was not found
    KeyNotFound,
    /// Node is underfull and needs rebalancing (contains the deleted value)
    NodeUnderfull(Vec<u8>),
}
