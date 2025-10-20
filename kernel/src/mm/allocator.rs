// Kernel Heap Allocator
// Provides kmalloc/kfree for dynamic memory allocation
// Uses Buddy System algorithm for efficient allocation

#![allow(dead_code)]

use spin::Mutex;

/// Minimum block size (64 bytes)
const MIN_BLOCK_SIZE: usize = 64;

/// Maximum block size (1 MB)
const MAX_BLOCK_SIZE: usize = 1048576;

/// Number of orders (64B to 1MB = 2^6 to 2^20 = 15 orders)
const NUM_ORDERS: usize = 15;

/// Free block node in the free list
#[repr(C)]
struct FreeBlock {
    size: usize,
    next: Option<*mut FreeBlock>,
}

/// Buddy allocator for kernel heap
pub struct BuddyAllocator {
    /// Free lists for each order (size = 2^order * MIN_BLOCK_SIZE)
    free_lists: [Option<*mut FreeBlock>; NUM_ORDERS],
    /// Start of heap
    heap_start: usize,
    /// End of heap
    heap_end: usize,
    /// Total allocated bytes
    allocated: usize,
}

/// Global allocator instance
static ALLOCATOR: Mutex<Option<BuddyAllocator>> = Mutex::new(None);

// Safety: BuddyAllocator is protected by a Mutex, so it's safe to send between threads
unsafe impl Send for BuddyAllocator {}

impl BuddyAllocator {
    /// Initialize allocator with heap range
    pub fn init(start: usize, size: usize) -> Self {
        let heap_end = start + size;

        let mut allocator = BuddyAllocator {
            free_lists: [None; NUM_ORDERS],
            heap_start: start,
            heap_end,
            allocated: 0,
        };

        // Add initial heap memory to appropriate free list
        // Find the largest order that fits the heap size
        let mut remaining_size = size;
        let mut current_addr = start;

        while remaining_size >= MIN_BLOCK_SIZE {
            // Find the largest block size that fits
            let mut order = NUM_ORDERS - 1;
            let mut block_size = MAX_BLOCK_SIZE;

            while block_size > remaining_size || block_size > MAX_BLOCK_SIZE {
                if order == 0 {
                    break;
                }
                order -= 1;
                block_size = MIN_BLOCK_SIZE << order;
            }

            // Add block to free list
            if block_size <= remaining_size && block_size >= MIN_BLOCK_SIZE {
                let block = current_addr as *mut FreeBlock;
                unsafe {
                    (*block).size = block_size;
                    (*block).next = allocator.free_lists[order];
                }
                allocator.free_lists[order] = Some(block);

                current_addr += block_size;
                remaining_size -= block_size;
            } else {
                break;
            }
        }

        allocator
    }

    /// Allocate memory block
    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        if size == 0 {
            return core::ptr::null_mut();
        }

        // Round up size to power of 2, minimum MIN_BLOCK_SIZE
        let actual_size = if size < MIN_BLOCK_SIZE {
            MIN_BLOCK_SIZE
        } else {
            size.next_power_of_two()
        };

        // Check if size exceeds maximum
        if actual_size > MAX_BLOCK_SIZE {
            return core::ptr::null_mut();
        }

        // Calculate order
        let order = (actual_size / MIN_BLOCK_SIZE).trailing_zeros() as usize;

        if order >= NUM_ORDERS {
            return core::ptr::null_mut();
        }

        // Search for free block in appropriate order
        if let Some(block) = self.find_free_block(order) {
            self.allocated += actual_size;

            // Zero allocated memory for security
            unsafe {
                core::ptr::write_bytes(block as *mut u8, 0, actual_size);
            }

            return block as *mut u8;
        }

        // No free block found
        core::ptr::null_mut()
    }

    /// Find or split blocks to get a free block of the requested order
    fn find_free_block(&mut self, order: usize) -> Option<*mut FreeBlock> {
        // Try to find a block in the requested order
        if let Some(block) = self.free_lists[order] {
            // Remove from free list
            unsafe {
                self.free_lists[order] = (*block).next;
            }
            return Some(block);
        }

        // No block in this order, try to split a larger block
        if order + 1 < NUM_ORDERS {
            if let Some(larger_block) = self.find_free_block(order + 1) {
                // Split the larger block into two buddies
                let block_size = MIN_BLOCK_SIZE << order;
                let buddy = (larger_block as usize + block_size) as *mut FreeBlock;

                // Add buddy to free list
                unsafe {
                    (*buddy).size = block_size;
                    (*buddy).next = self.free_lists[order];
                }
                self.free_lists[order] = Some(buddy);

                // Update size of the block we're returning
                unsafe {
                    (*larger_block).size = block_size;
                }

                return Some(larger_block);
            }
        }

        None
    }

    /// Free memory block
    pub fn free(&mut self, ptr: *mut u8, size: usize) {
        if ptr.is_null() || size == 0 {
            return;
        }

        let addr = ptr as usize;

        // Validate address is within heap
        if addr < self.heap_start || addr >= self.heap_end {
            return;
        }

        // Round up size to power of 2, minimum MIN_BLOCK_SIZE
        let actual_size = if size < MIN_BLOCK_SIZE {
            MIN_BLOCK_SIZE
        } else {
            size.next_power_of_two()
        };

        if actual_size > MAX_BLOCK_SIZE {
            return;
        }

        // Calculate order
        let order = (actual_size / MIN_BLOCK_SIZE).trailing_zeros() as usize;

        if order >= NUM_ORDERS {
            return;
        }

        self.allocated -= actual_size;

        // Try to merge with buddy
        self.free_and_merge(addr, order);
    }

    /// Free block and merge with buddy if possible
    fn free_and_merge(&mut self, addr: usize, order: usize) {
        let block_size = MIN_BLOCK_SIZE << order;

        // Calculate buddy address using XOR
        let buddy_addr = addr ^ block_size;

        // Check if buddy is free and within heap bounds
        if order + 1 < NUM_ORDERS && buddy_addr >= self.heap_start && buddy_addr < self.heap_end {
            // Search for buddy in free list
            if self.remove_from_free_list(buddy_addr, order) {
                // Buddy found and removed, merge blocks
                let merged_addr = if addr < buddy_addr { addr } else { buddy_addr };

                // Recursively try to merge at next order
                self.free_and_merge(merged_addr, order + 1);
                return;
            }
        }

        // Cannot merge, add to free list
        let block = addr as *mut FreeBlock;
        unsafe {
            (*block).size = block_size;
            (*block).next = self.free_lists[order];
        }
        self.free_lists[order] = Some(block);
    }

    /// Remove a block from free list if it exists
    fn remove_from_free_list(&mut self, addr: usize, order: usize) -> bool {
        let mut current = self.free_lists[order];
        let mut prev: Option<*mut FreeBlock> = None;

        while let Some(block) = current {
            if block as usize == addr {
                // Found the block, remove it
                unsafe {
                    if let Some(prev_block) = prev {
                        (*prev_block).next = (*block).next;
                    } else {
                        self.free_lists[order] = (*block).next;
                    }
                }
                return true;
            }

            prev = Some(block);
            unsafe {
                current = (*block).next;
            }
        }

        false
    }

    /// Get allocated memory in bytes
    pub fn allocated_bytes(&self) -> usize {
        self.allocated
    }
}

/// Initialize the global allocator
pub fn init_allocator(start: usize, size: usize) {
    let allocator = BuddyAllocator::init(start, size);
    *ALLOCATOR.lock() = Some(allocator);
}

/// Allocate memory (thread-safe public API)
/// Returns a pointer to allocated memory or null if out of memory
pub fn kmalloc(size: usize) -> *mut u8 {
    let mut allocator_guard = ALLOCATOR.lock();

    if let Some(allocator) = allocator_guard.as_mut() {
        let ptr = allocator.alloc(size);

        if ptr.is_null() {
            // Out of memory - log error
            // TODO: Add logging when logging infrastructure is available
            // kprintln!("[MM] ERROR: Out of memory, failed to allocate {} bytes", size);
        } else {
            // Log successful allocation
            // TODO: Add logging when logging infrastructure is available
            // kprintln!("[MM] Allocated {} bytes at 0x{:p}", size, ptr);
        }

        ptr
    } else {
        // Allocator not initialized
        core::ptr::null_mut()
    }
}

/// Free memory (thread-safe public API)
pub fn kfree(ptr: *mut u8, size: usize) {
    if ptr.is_null() {
        return;
    }

    let mut allocator_guard = ALLOCATOR.lock();

    if let Some(allocator) = allocator_guard.as_mut() {
        allocator.free(ptr, size);

        // Log deallocation
        // TODO: Add logging when logging infrastructure is available
        // kprintln!("[MM] Freed {} bytes from 0x{:p}", size, ptr);
    }
}

/// Get total allocated memory in bytes
pub fn allocated_bytes() -> usize {
    let allocator_guard = ALLOCATOR.lock();

    if let Some(allocator) = allocator_guard.as_ref() {
        allocator.allocated_bytes()
    } else {
        0
    }
}
