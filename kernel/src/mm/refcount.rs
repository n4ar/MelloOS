// Page Reference Counting for Copy-on-Write
//
// This module provides atomic reference counting for physical pages,
// enabling safe sharing between processes (e.g., for fork with COW).

use crate::mm::{pmm::PhysicalMemoryManager, PhysAddr};
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Page reference count entry
///
/// Tracks how many processes/mappings reference a physical page.
/// Uses atomic operations for SMP-safe reference counting.
struct PageRefEntry {
    /// Atomic reference count
    count: AtomicUsize,
}

impl PageRefEntry {
    /// Create a new reference count entry with initial count
    fn new(initial_count: usize) -> Self {
        Self {
            count: AtomicUsize::new(initial_count),
        }
    }

    /// Increment the reference count
    ///
    /// Returns the new reference count
    fn inc(&self) -> usize {
        self.count.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Decrement the reference count
    ///
    /// Returns the new reference count (0 means the page should be freed)
    fn dec(&self) -> usize {
        let old_count = self.count.fetch_sub(1, Ordering::AcqRel);
        if old_count == 0 {
            panic!("PageRefEntry: refcount underflow!");
        }
        old_count - 1
    }

    /// Get the current reference count
    fn get(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }
}

/// Page Reference Counter
///
/// Manages reference counts for all shared physical pages.
/// Uses a BTreeMap for efficient lookup and a SpinLock for SMP safety.
pub struct PageRefcount {
    /// Map from physical address to reference count
    /// BTreeMap is used instead of HashMap for deterministic iteration
    counts: SpinLock<BTreeMap<PhysAddr, PageRefEntry>>,
}

impl PageRefcount {
    /// Create a new empty page reference counter
    pub const fn new() -> Self {
        Self {
            counts: SpinLock::new(BTreeMap::new()),
        }
    }

    /// Increment reference count for a page
    ///
    /// If the page doesn't have a refcount entry, creates one with count 2
    /// (assuming the page was already referenced once before this call).
    ///
    /// # Arguments
    /// * `page` - Physical address of the page (must be 4KB aligned)
    ///
    /// # Returns
    /// The new reference count
    ///
    /// # Panics
    /// Panics if the page address is not 4KB aligned
    pub fn inc_refcount(&self, page: PhysAddr) -> usize {
        // Validate alignment
        assert_eq!(page % 4096, 0, "Page address must be 4KB aligned");

        let mut counts = self.counts.lock();

        if let Some(entry) = counts.get(&page) {
            // Page already has a refcount, increment it
            entry.inc()
        } else {
            // First time sharing this page, create entry with count 2
            // (1 for original owner + 1 for new reference)
            counts.insert(page, PageRefEntry::new(2));
            2
        }
    }

    /// Decrement reference count for a page
    ///
    /// If the refcount reaches zero, the page is freed using the provided PMM.
    ///
    /// # Arguments
    /// * `page` - Physical address of the page (must be 4KB aligned)
    /// * `pmm` - Physical memory manager for freeing the page if refcount reaches 0
    ///
    /// # Returns
    /// The new reference count (0 means the page was freed)
    ///
    /// # Panics
    /// Panics if the page address is not 4KB aligned or if the page has no refcount entry
    pub fn dec_refcount(&self, page: PhysAddr, pmm: &mut PhysicalMemoryManager) -> usize {
        // Validate alignment
        assert_eq!(page % 4096, 0, "Page address must be 4KB aligned");

        let mut counts = self.counts.lock();

        if let Some(entry) = counts.get(&page) {
            let new_count = entry.dec();

            if new_count == 0 {
                // Last reference removed, free the page
                counts.remove(&page);
                drop(counts); // Release lock before freeing

                // Free the physical page
                pmm.free_frame(page);

                crate::serial_println!(
                    "[REFCOUNT] Freed page at phys={:#x} (refcount reached 0)",
                    page
                );
            }

            new_count
        } else {
            panic!(
                "Attempted to decrement refcount for page {:#x} with no entry",
                page
            );
        }
    }

    /// Get the current reference count for a page
    ///
    /// # Arguments
    /// * `page` - Physical address of the page (must be 4KB aligned)
    ///
    /// # Returns
    /// The current reference count, or 1 if the page has no refcount entry
    /// (pages without entries are assumed to have a single reference)
    ///
    /// # Panics
    /// Panics if the page address is not 4KB aligned
    pub fn get_refcount(&self, page: PhysAddr) -> usize {
        // Validate alignment
        assert_eq!(page % 4096, 0, "Page address must be 4KB aligned");

        let counts = self.counts.lock();

        if let Some(entry) = counts.get(&page) {
            entry.get()
        } else {
            // Page has no refcount entry, assume single reference
            1
        }
    }

    /// Check if a page is shared (refcount > 1)
    ///
    /// # Arguments
    /// * `page` - Physical address of the page (must be 4KB aligned)
    ///
    /// # Returns
    /// true if the page has multiple references, false otherwise
    pub fn is_shared(&self, page: PhysAddr) -> bool {
        self.get_refcount(page) > 1
    }

    /// Remove a page from the refcount tracking without freeing it
    ///
    /// This is useful when a page is being freed through other means
    /// (e.g., process cleanup) and we just need to remove the tracking entry.
    ///
    /// # Arguments
    /// * `page` - Physical address of the page (must be 4KB aligned)
    pub fn remove_tracking(&self, page: PhysAddr) {
        // Validate alignment
        assert_eq!(page % 4096, 0, "Page address must be 4KB aligned");

        let mut counts = self.counts.lock();
        counts.remove(&page);
    }

    /// Get the total number of pages being tracked
    ///
    /// This is useful for debugging and statistics.
    ///
    /// # Returns
    /// The number of pages with refcount entries
    pub fn tracked_pages(&self) -> usize {
        let counts = self.counts.lock();
        counts.len()
    }
}

/// Global page reference counter
///
/// This is the single global instance used throughout the kernel for
/// tracking page reference counts for COW and shared mappings.
pub static PAGE_REFCOUNT: PageRefcount = PageRefcount::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refcount_basic() {
        let refcount = PageRefcount::new();
        let page = 0x1000; // 4KB aligned

        // Initial refcount should be 1 (no entry)
        assert_eq!(refcount.get_refcount(page), 1);

        // Increment creates entry with count 2
        assert_eq!(refcount.inc_refcount(page), 2);
        assert_eq!(refcount.get_refcount(page), 2);

        // Increment again
        assert_eq!(refcount.inc_refcount(page), 3);
        assert_eq!(refcount.get_refcount(page), 3);

        // Check if shared
        assert!(refcount.is_shared(page));
    }

    #[test]
    fn test_tracked_pages() {
        let refcount = PageRefcount::new();

        assert_eq!(refcount.tracked_pages(), 0);

        refcount.inc_refcount(0x1000);
        assert_eq!(refcount.tracked_pages(), 1);

        refcount.inc_refcount(0x2000);
        assert_eq!(refcount.tracked_pages(), 2);

        refcount.remove_tracking(0x1000);
        assert_eq!(refcount.tracked_pages(), 1);
    }

    #[test]
    #[should_panic(expected = "Page address must be 4KB aligned")]
    fn test_unaligned_address() {
        let refcount = PageRefcount::new();
        refcount.inc_refcount(0x1001); // Not 4KB aligned
    }
}
