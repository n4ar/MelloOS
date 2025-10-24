//! MelloFS Space Allocator
//!
//! Manages free space using a B-tree of free extents.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Free extent (start_lba, length)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FreeExtent {
    pub start_lba: u64,
    pub length: u32,
}

impl FreeExtent {
    pub fn new(start_lba: u64, length: u32) -> Self {
        Self { start_lba, length }
    }
    
    pub fn end_lba(&self) -> u64 {
        self.start_lba + self.length as u64
    }
}

/// Allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocStrategy {
    /// First-fit: allocate from first extent that fits
    FirstFit,
    /// Best-fit: allocate from smallest extent that fits
    BestFit,
}

/// Space allocator
pub struct SpaceAllocator {
    /// Free extents (sorted by start_lba)
    free_extents: BTreeMap<u64, FreeExtent>,
    /// Total free blocks
    free_blocks: u64,
    /// Allocation strategy
    strategy: AllocStrategy,
    /// Pending allocations (delayed allocation)
    pending: Vec<PendingAlloc>,
}

/// Pending allocation (delayed allocation)
#[derive(Debug, Clone)]
struct PendingAlloc {
    /// Number of blocks requested
    blocks: u32,
    /// Allocated extent (once assigned)
    allocated: Option<FreeExtent>,
}

impl SpaceAllocator {
    /// Create a new space allocator
    pub fn new(strategy: AllocStrategy) -> Self {
        Self {
            free_extents: BTreeMap::new(),
            free_blocks: 0,
            strategy,
            pending: Vec::new(),
        }
    }
    
    /// Initialize with a single large free extent
    pub fn init(&mut self, start_lba: u64, total_blocks: u64) {
        let extent = FreeExtent::new(start_lba, total_blocks as u32);
        self.free_extents.insert(start_lba, extent);
        self.free_blocks = total_blocks;
    }
    
    /// Add a free extent
    pub fn add_free_extent(&mut self, extent: FreeExtent) {
        self.free_extents.insert(extent.start_lba, extent);
        self.free_blocks += extent.length as u64;
        
        // Try to coalesce with adjacent extents
        self.coalesce_at(extent.start_lba);
    }
    
    /// Allocate blocks immediately
    ///
    /// Returns the allocated extent, or None if not enough space.
    pub fn allocate(&mut self, blocks: u32) -> Option<FreeExtent> {
        if blocks == 0 {
            return None;
        }
        
        match self.strategy {
            AllocStrategy::FirstFit => self.allocate_first_fit(blocks),
            AllocStrategy::BestFit => self.allocate_best_fit(blocks),
        }
    }
    
    /// Allocate using first-fit strategy
    fn allocate_first_fit(&mut self, blocks: u32) -> Option<FreeExtent> {
        // Find first extent that fits
        let candidate = self.free_extents
            .iter()
            .find(|(_, extent)| extent.length >= blocks)
            .map(|(&start, &extent)| (start, extent))?;
        
        self.split_and_allocate(candidate.0, blocks)
    }
    
    /// Allocate using best-fit strategy
    fn allocate_best_fit(&mut self, blocks: u32) -> Option<FreeExtent> {
        // Find smallest extent that fits
        let candidate = self.free_extents
            .iter()
            .filter(|(_, extent)| extent.length >= blocks)
            .min_by_key(|(_, extent)| extent.length)
            .map(|(&start, &extent)| (start, extent))?;
        
        self.split_and_allocate(candidate.0, blocks)
    }
    
    /// Split an extent and allocate from it
    fn split_and_allocate(&mut self, start_lba: u64, blocks: u32) -> Option<FreeExtent> {
        let extent = self.free_extents.remove(&start_lba)?;
        
        let allocated = FreeExtent::new(extent.start_lba, blocks);
        
        // If there's remaining space, add it back
        if extent.length > blocks {
            let remaining = FreeExtent::new(
                extent.start_lba + blocks as u64,
                extent.length - blocks,
            );
            self.free_extents.insert(remaining.start_lba, remaining);
        }
        
        self.free_blocks -= blocks as u64;
        
        Some(allocated)
    }
    
    /// Free an allocated extent
    pub fn free(&mut self, extent: FreeExtent) {
        self.add_free_extent(extent);
    }
    
    /// Delayed allocation: reserve blocks without assigning physical location
    ///
    /// Returns an allocation ID that can be used to commit later.
    pub fn delayed_alloc(&mut self, blocks: u32) -> Result<usize, &'static str> {
        if self.free_blocks < blocks as u64 {
            return Err("Not enough free space");
        }
        
        let alloc_id = self.pending.len();
        self.pending.push(PendingAlloc {
            blocks,
            allocated: None,
        });
        
        // Reserve the blocks
        self.free_blocks -= blocks as u64;
        
        Ok(alloc_id)
    }
    
    /// Commit a delayed allocation
    ///
    /// Assigns physical blocks to a previously reserved allocation.
    pub fn commit_delayed_alloc(&mut self, alloc_id: usize) -> Result<FreeExtent, &'static str> {
        if alloc_id >= self.pending.len() {
            return Err("Invalid allocation ID");
        }
        
        let pending = &mut self.pending[alloc_id];
        
        if let Some(extent) = pending.allocated {
            return Ok(extent);
        }
        
        // Allocate physical blocks
        let extent = self.allocate(pending.blocks)
            .ok_or("Failed to allocate blocks")?;
        
        pending.allocated = Some(extent);
        
        Ok(extent)
    }
    
    /// Cancel a delayed allocation
    pub fn cancel_delayed_alloc(&mut self, alloc_id: usize) -> Result<(), &'static str> {
        if alloc_id >= self.pending.len() {
            return Err("Invalid allocation ID");
        }
        
        let pending = &self.pending[alloc_id];
        
        // Return reserved blocks
        self.free_blocks += pending.blocks as u64;
        
        // If already allocated, free the extent
        if let Some(extent) = pending.allocated {
            self.add_free_extent(extent);
        }
        
        Ok(())
    }
    
    /// Coalesce adjacent free extents
    fn coalesce_at(&mut self, start_lba: u64) {
        let extent = match self.free_extents.get(&start_lba) {
            Some(&e) => e,
            None => return,
        };
        
        // Try to merge with next extent
        let next_lba = extent.end_lba();
        if let Some(&next_extent) = self.free_extents.get(&next_lba) {
            // Remove both extents
            self.free_extents.remove(&start_lba);
            self.free_extents.remove(&next_lba);
            
            // Create merged extent
            let merged = FreeExtent::new(
                extent.start_lba,
                extent.length + next_extent.length,
            );
            
            self.free_extents.insert(merged.start_lba, merged);
            
            // Try to coalesce again
            self.coalesce_at(merged.start_lba);
        }
        
        // Try to merge with previous extent
        if let Some((&prev_lba, &prev_extent)) = self.free_extents
            .range(..start_lba)
            .next_back()
        {
            if prev_extent.end_lba() == start_lba {
                // Remove both extents
                self.free_extents.remove(&prev_lba);
                self.free_extents.remove(&start_lba);
                
                // Create merged extent
                let merged = FreeExtent::new(
                    prev_extent.start_lba,
                    prev_extent.length + extent.length,
                );
                
                self.free_extents.insert(merged.start_lba, merged);
            }
        }
    }
    
    /// Get total free blocks
    pub fn free_blocks(&self) -> u64 {
        self.free_blocks
    }
    
    /// Get number of free extents
    pub fn num_free_extents(&self) -> usize {
        self.free_extents.len()
    }
    
    /// Get all free extents (for debugging)
    pub fn get_free_extents(&self) -> Vec<FreeExtent> {
        self.free_extents.values().copied().collect()
    }
}

// Tests would go here but are omitted for kernel code
