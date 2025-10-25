//! Simple allocator for fs_test

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

/// Simple bump allocator
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: UnsafeCell<usize>,
}

// SAFETY: We're single-threaded in userspace
unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: UnsafeCell::new(0),
        }
    }
    
    pub fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        unsafe {
            *self.next.get() = heap_start;
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let next_ptr = self.next.get();
        let alloc_start = align_up(*next_ptr, layout.align());
        let alloc_end = alloc_start + layout.size();
        
        if alloc_end > self.heap_end {
            core::ptr::null_mut()
        } else {
            // Update next pointer
            *next_ptr = alloc_end;
            alloc_start as *mut u8
        }
    }
    
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

/// Initialize the allocator with a static buffer
pub fn init() {
    static mut HEAP: [u8; 64 * 1024] = [0; 64 * 1024]; // 64KB heap
    unsafe {
        let heap_start = HEAP.as_ptr() as usize;
        let heap_size = HEAP.len();
        let allocator = &ALLOCATOR as *const BumpAllocator as *mut BumpAllocator;
        (*allocator).init(heap_start, heap_size);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
