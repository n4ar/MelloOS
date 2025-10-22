use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::cell::UnsafeCell;

struct BumpAllocator {
    heap_start: UnsafeCell<usize>,
    heap_end: UnsafeCell<usize>,
    next: UnsafeCell<usize>,
}

unsafe impl Sync for BumpAllocator {}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: UnsafeCell::new(0),
            heap_end: UnsafeCell::new(0),
            next: UnsafeCell::new(0),
        }
    }

    unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        *self.heap_start.get() = heap_start;
        *self.heap_end.get() = heap_start + heap_size;
        *self.next.get() = heap_start;
    }

    unsafe fn alloc_inner(&self, layout: Layout) -> *mut u8 {
        let next = *self.next.get();
        let heap_end = *self.heap_end.get();
        
        let alloc_start = align_up(next, layout.align());
        let alloc_end = alloc_start.saturating_add(layout.size());

        if alloc_end > heap_end {
            null_mut()
        } else {
            *self.next.get() = alloc_end;
            alloc_start as *mut u8
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_inner(layout)
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

pub fn init_heap() {
    unsafe {
        ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP.len());
    }
}
