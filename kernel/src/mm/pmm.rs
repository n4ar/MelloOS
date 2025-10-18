// Physical Memory Manager
// Manages physical memory frames (4KB blocks)

#![allow(dead_code)]

use crate::mm::{phys_to_virt, PhysAddr};
use limine::memory_map::EntryType;
use limine::response::MemoryMapResponse;

/// Size of a physical frame (4KB page)
pub const FRAME_SIZE: usize = 4096;

/// Physical Memory Manager
/// Uses a bitmap allocator to track free and used frames
pub struct PhysicalMemoryManager {
    /// Bitmap where each bit represents one frame
    /// 0 = free, 1 = used
    bitmap: &'static mut [u8],
    /// Total number of frames in the system
    total_frames: usize,
    /// Number of free frames available
    free_frames: usize,
    /// Start of usable memory (physical address)
    memory_start: PhysAddr,
    /// End of usable memory (physical address)
    memory_end: PhysAddr,
    /// Last allocated frame index (for faster sequential allocation)
    last_alloc: usize,
}

impl PhysicalMemoryManager {
    /// Initialize the Physical Memory Manager from Limine memory map
    /// 
    /// This function:
    /// - Parses the memory map and filters for Usable memory only
    /// - Calculates total frames and allocates bitmap
    /// - Marks kernel image and page tables as used
    /// - Logs total and usable memory in MB
    pub fn init(
        memory_map: &MemoryMapResponse,
        kernel_start: PhysAddr,
        kernel_end: PhysAddr,
    ) -> Self {
        // Find the highest usable memory address and calculate total frames
        let mut highest_addr = 0usize;
        
        for entry in memory_map.entries() {
            // Only consider Usable memory regions
            if entry.entry_type == EntryType::USABLE {
                let entry_end = entry.base as usize + entry.length as usize;
                if entry_end > highest_addr {
                    highest_addr = entry_end;
                }
            }
        }
        
        let total_frames = highest_addr / FRAME_SIZE;
        let bitmap_size = (total_frames + 7) / 8; // Round up to nearest byte
        
        // Find a suitable location for the bitmap in usable memory
        // We'll place it after the kernel
        let bitmap_start = (kernel_end + FRAME_SIZE - 1) & !(FRAME_SIZE - 1); // Align to frame
        let bitmap_virt = phys_to_virt(bitmap_start);
        
        // Create bitmap slice
        let bitmap = unsafe {
            core::slice::from_raw_parts_mut(bitmap_virt as *mut u8, bitmap_size)
        };
        
        // Initialize bitmap - mark all frames as used initially
        for byte in bitmap.iter_mut() {
            *byte = 0xFF;
        }
        
        let mut pmm = PhysicalMemoryManager {
            bitmap,
            total_frames,
            free_frames: 0,
            memory_start: 0,
            memory_end: highest_addr,
            last_alloc: 0,
        };
        
        // Mark usable memory regions as free
        for entry in memory_map.entries() {
            if entry.entry_type == EntryType::USABLE {
                let start_frame = (entry.base as usize) / FRAME_SIZE;
                let end_frame = ((entry.base as usize + entry.length as usize) + FRAME_SIZE - 1) / FRAME_SIZE;
                
                for frame in start_frame..end_frame {
                    if frame < total_frames {
                        pmm.mark_frame_free(frame);
                    }
                }
            }
        }
        
        // Mark kernel image as used
        let kernel_start_frame = kernel_start / FRAME_SIZE;
        let kernel_end_frame = (kernel_end + FRAME_SIZE - 1) / FRAME_SIZE;
        
        for frame in kernel_start_frame..kernel_end_frame {
            if frame < total_frames {
                pmm.mark_frame_used(frame);
            }
        }
        
        // Mark bitmap itself as used
        let bitmap_end = bitmap_start + bitmap_size;
        let bitmap_start_frame = bitmap_start / FRAME_SIZE;
        let bitmap_end_frame = (bitmap_end + FRAME_SIZE - 1) / FRAME_SIZE;
        
        for frame in bitmap_start_frame..bitmap_end_frame {
            if frame < total_frames {
                pmm.mark_frame_used(frame);
            }
        }
        
        // TODO: Log memory information once logging is available
        // let total_mb = (total_frames * FRAME_SIZE) / (1024 * 1024);
        // let usable_mb = total_usable_bytes / (1024 * 1024);
        // let free_mb = (pmm.free_frames * FRAME_SIZE) / (1024 * 1024);
        
        pmm
    }
    
    /// Mark a frame as free in the bitmap
    fn mark_frame_free(&mut self, frame: usize) {
        let byte_index = frame / 8;
        let bit_index = frame % 8;
        
        if byte_index < self.bitmap.len() {
            let was_used = (self.bitmap[byte_index] & (1 << bit_index)) != 0;
            self.bitmap[byte_index] &= !(1 << bit_index);
            
            if was_used {
                self.free_frames += 1;
            }
        }
    }
    
    /// Mark a frame as used in the bitmap
    fn mark_frame_used(&mut self, frame: usize) {
        let byte_index = frame / 8;
        let bit_index = frame % 8;
        
        if byte_index < self.bitmap.len() {
            let was_free = (self.bitmap[byte_index] & (1 << bit_index)) == 0;
            self.bitmap[byte_index] |= 1 << bit_index;
            
            if was_free {
                self.free_frames -= 1;
            }
        }
    }
    
    /// Check if a frame is free
    fn is_frame_free(&self, frame: usize) -> bool {
        let byte_index = frame / 8;
        let bit_index = frame % 8;
        
        if byte_index < self.bitmap.len() {
            (self.bitmap[byte_index] & (1 << bit_index)) == 0
        } else {
            false
        }
    }
}

impl PhysicalMemoryManager {
    /// Allocate a physical frame
    /// 
    /// Returns the physical address of the allocated frame, or None if out of memory.
    /// The allocated frame is zeroed for security.
    pub fn alloc_frame(&mut self) -> Option<PhysAddr> {
        // Check if we have any free frames
        if self.free_frames == 0 {
            // TODO: Log error once logging is available
            return None;
        }
        
        // Scan bitmap starting from last_alloc for faster sequential allocation
        let start_frame = self.last_alloc;
        
        for offset in 0..self.total_frames {
            let frame = (start_frame + offset) % self.total_frames;
            
            if self.is_frame_free(frame) {
                // Mark frame as used
                self.mark_frame_used(frame);
                self.last_alloc = frame;
                
                // Calculate physical address
                let phys_addr = frame * FRAME_SIZE;
                
                // Zero the frame for security
                let virt_addr = phys_to_virt(phys_addr);
                unsafe {
                    core::ptr::write_bytes(virt_addr as *mut u8, 0, FRAME_SIZE);
                }
                
                return Some(phys_addr);
            }
        }
        
        // Should never reach here if free_frames > 0
        None
    }
    
    /// Get total memory in MB
    pub fn total_memory_mb(&self) -> usize {
        (self.total_frames * FRAME_SIZE) / (1024 * 1024)
    }
    
    /// Get free memory in MB
    pub fn free_memory_mb(&self) -> usize {
        (self.free_frames * FRAME_SIZE) / (1024 * 1024)
    }
}

impl PhysicalMemoryManager {
    /// Free a physical frame
    /// 
    /// Marks the frame at the given physical address as free and available for reuse.
    pub fn free_frame(&mut self, phys_addr: PhysAddr) {
        // Validate address alignment
        if phys_addr % FRAME_SIZE != 0 {
            return;
        }
        
        let frame = phys_addr / FRAME_SIZE;
        
        // Validate frame is within bounds
        if frame >= self.total_frames {
            return;
        }
        
        // Check if frame is already free
        if self.is_frame_free(frame) {
            return;
        }
        
        // Mark frame as free
        self.mark_frame_free(frame);
    }
}

impl PhysicalMemoryManager {
    /// Allocate contiguous physical frames for DMA
    /// 
    /// Finds and allocates a contiguous block of frames with the specified alignment.
    /// Returns the physical address of the first frame, or None if allocation fails.
    /// 
    /// # Arguments
    /// * `count` - Number of contiguous frames to allocate
    /// * `align` - Alignment requirement in bytes (must be power of 2)
    pub fn alloc_contiguous(&mut self, count: usize, align: usize) -> Option<PhysAddr> {
        // Validate alignment is power of 2
        if align == 0 || (align & (align - 1)) != 0 {
            return None;
        }
        
        // Check if we have enough free frames
        if self.free_frames < count {
            return None;
        }
        
        let align_frames = align / FRAME_SIZE;
        
        // Scan for contiguous free frames with proper alignment
        let mut start_frame = 0;
        
        while start_frame < self.total_frames {
            // Align start_frame
            if align_frames > 1 {
                start_frame = (start_frame + align_frames - 1) & !(align_frames - 1);
            }
            
            if start_frame + count > self.total_frames {
                break;
            }
            
            // Check if all frames in range are free
            let mut all_free = true;
            for offset in 0..count {
                if !self.is_frame_free(start_frame + offset) {
                    all_free = false;
                    start_frame = start_frame + offset + 1;
                    break;
                }
            }
            
            if all_free {
                // Allocate all frames in the range
                for offset in 0..count {
                    self.mark_frame_used(start_frame + offset);
                }
                
                let phys_addr = start_frame * FRAME_SIZE;
                
                // Zero all frames for security
                let virt_addr = phys_to_virt(phys_addr);
                unsafe {
                    core::ptr::write_bytes(virt_addr as *mut u8, 0, count * FRAME_SIZE);
                }
                
                return Some(phys_addr);
            }
        }
        
        None
    }
}
