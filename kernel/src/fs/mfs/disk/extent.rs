//! MelloFS Extent Manager
//!
//! Manages file extents (contiguous block ranges).

use super::keys::{ExtentKey, ExtentVal};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Extent manager for tracking file extents
pub struct ExtentManager {
    /// Block size
    block_size: u32,
    /// In-memory extent cache (ino -> offset -> extent)
    cache: BTreeMap<u64, BTreeMap<u64, ExtentVal>>,
}

impl ExtentManager {
    /// Create a new extent manager
    pub fn new(block_size: u32) -> Self {
        Self {
            block_size,
            cache: BTreeMap::new(),
        }
    }

    /// Allocate a new extent for a file
    ///
    /// Returns the extent key and value.
    pub fn allocate_extent(
        &mut self,
        ino: u64,
        file_offset: u64,
        phys_lba: u64,
        length: u32,
    ) -> Result<(ExtentKey, ExtentVal), &'static str> {
        // Validate alignment
        if file_offset % self.block_size as u64 != 0 {
            return Err("File offset not block-aligned");
        }

        // Create extent
        let key = ExtentKey::new(ino, file_offset);
        let val = ExtentVal::new(phys_lba, length);

        // Add to cache
        self.cache
            .entry(ino)
            .or_insert_with(BTreeMap::new)
            .insert(file_offset, val);

        Ok((key, val))
    }

    /// Lookup extent for a given file offset
    ///
    /// Returns the extent that contains the offset, or None if not found.
    pub fn lookup_extent(&self, ino: u64, file_offset: u64) -> Option<(u64, ExtentVal)> {
        let extents = self.cache.get(&ino)?;

        // Find the extent that contains this offset
        // We need the largest key <= file_offset
        let mut result = None;
        for (&offset, &extent) in extents.iter() {
            if offset <= file_offset {
                let extent_end = offset + (extent.length as u64 * self.block_size as u64);
                if file_offset < extent_end {
                    result = Some((offset, extent));
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// Extend an existing extent
    ///
    /// Attempts to extend the extent at the given offset by additional blocks.
    pub fn extend_extent(
        &mut self,
        ino: u64,
        file_offset: u64,
        additional_blocks: u32,
    ) -> Result<(), &'static str> {
        let extents = self.cache.get_mut(&ino).ok_or("Inode not found")?;

        let extent = extents.get_mut(&file_offset).ok_or("Extent not found")?;

        extent.length += additional_blocks;

        Ok(())
    }

    /// Free an extent
    ///
    /// Removes the extent from tracking and returns it for deallocation.
    pub fn free_extent(&mut self, ino: u64, file_offset: u64) -> Result<ExtentVal, &'static str> {
        let extents = self.cache.get_mut(&ino).ok_or("Inode not found")?;

        extents.remove(&file_offset).ok_or("Extent not found")
    }

    /// Get all extents for an inode
    pub fn get_all_extents(&self, ino: u64) -> Vec<(u64, ExtentVal)> {
        self.cache
            .get(&ino)
            .map(|extents| {
                extents
                    .iter()
                    .map(|(&offset, &val)| (offset, val))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Remove all extents for an inode
    pub fn remove_inode(&mut self, ino: u64) -> Vec<ExtentVal> {
        self.cache
            .remove(&ino)
            .map(|extents| extents.into_iter().map(|(_, val)| val).collect())
            .unwrap_or_default()
    }

    /// Coalesce adjacent extents
    ///
    /// Merges adjacent extents to reduce fragmentation.
    pub fn coalesce_extents(&mut self, ino: u64) -> usize {
        let extents = match self.cache.get_mut(&ino) {
            Some(e) => e,
            None => return 0,
        };

        let mut coalesced = 0;
        let mut to_remove = Vec::new();
        let mut to_add = Vec::new();

        let entries: Vec<_> = extents.iter().map(|(&k, &v)| (k, v)).collect();

        for i in 0..entries.len().saturating_sub(1) {
            let (offset1, extent1) = entries[i];
            let (offset2, extent2) = entries[i + 1];

            // Check if extents are adjacent in both file and physical space
            let file_end1 = offset1 + (extent1.length as u64 * self.block_size as u64);
            let phys_end1 = extent1.phys_lba + extent1.length as u64;

            if file_end1 == offset2 && phys_end1 == extent2.phys_lba {
                // Extents can be coalesced
                let mut merged = extent1;
                merged.length += extent2.length;

                to_remove.push(offset1);
                to_remove.push(offset2);
                to_add.push((offset1, merged));
                coalesced += 1;
            }
        }

        // Apply changes
        for offset in to_remove {
            extents.remove(&offset);
        }
        for (offset, extent) in to_add {
            extents.insert(offset, extent);
        }

        coalesced
    }

    /// Calculate total blocks used by an inode
    pub fn total_blocks(&self, ino: u64) -> u64 {
        self.cache
            .get(&ino)
            .map(|extents| extents.values().map(|e| e.length as u64).sum())
            .unwrap_or(0)
    }
}

// Tests would go here but are omitted for kernel code
