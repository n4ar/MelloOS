//! Simple bump allocator for userspace

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
        }
    }

    fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = alloc_start + layout.size();

        if alloc_end > self.heap_end {
            null_mut()
        } else {
            let ptr = alloc_start as *mut u8;
            core::ptr::write_volatile(&self.next as *const usize as *mut usize, alloc_end);
            ptr
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

static mut HEAP: [u8; 64 * 1024] = [0; 64 * 1024];

#[no_mangle]
pub extern "C" fn init_heap() {
    unsafe {
        let heap_start = HEAP.as_ptr() as usize;
        let heap_size = HEAP.len();
        core::ptr::write_volatile(&ALLOCATOR as *const BumpAllocator as *mut BumpAllocator, BumpAllocator {
            heap_start,
            heap_end: heap_start + heap_size,
            next: heap_start,
        });
    }
}
