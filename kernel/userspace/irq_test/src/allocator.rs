use core::alloc::{GlobalAlloc, Layout};

const HEAP_SIZE: usize = 64 * 1024; // 64 KB heap
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut HEAP_POS: usize = 0;

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Align the current position
        let pos = (HEAP_POS + align - 1) & !(align - 1);

        // Check if we have enough space
        if pos + size > HEAP_SIZE {
            return core::ptr::null_mut();
        }

        HEAP_POS = pos + size;
        HEAP.as_mut_ptr().add(pos)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't support deallocation
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

pub fn init_heap() {
    // Heap is statically allocated, no initialization needed
}
