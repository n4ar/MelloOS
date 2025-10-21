//! Simple bump allocator for userspace
//!
//! Provides a basic heap allocator for the terminal emulator.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

/// Simple bump allocator
pub struct BumpAllocator {
    heap: UnsafeCell<[u8; HEAP_SIZE]>,
    next: UnsafeCell<usize>,
}

const HEAP_SIZE: usize = 1024 * 1024; // 1 MB heap

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            heap: UnsafeCell::new([0; HEAP_SIZE]),
            next: UnsafeCell::new(0),
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let next = self.next.get();
        let heap = self.heap.get();

        let align = layout.align();
        let size = layout.size();

        // Align the next pointer
        let current = *next;
        let aligned = (current + align - 1) & !(align - 1);

        // Check if we have enough space
        if aligned + size > HEAP_SIZE {
            return ptr::null_mut();
        }

        // Update next pointer
        *next = aligned + size;

        // Return pointer to allocated memory
        (*heap).as_mut_ptr().add(aligned)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
        // This is acceptable for a simple terminal emulator
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();
