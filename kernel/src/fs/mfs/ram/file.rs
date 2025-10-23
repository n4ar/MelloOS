//! MelloFS RAM File Operations

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use core::sync::atomic::Ordering;
use crate::fs::vfs::superblock::FsError;
use crate::fs::mfs::ram::inode::{RamInode, InodeKind, FileData};

impl RamInode {
    /// Read data from file at offset
    pub fn file_read_at(&self, off: u64, dst: &mut [u8]) -> Result<usize, FsError> {
        // Lock file data
        let data = self.data.lock();
        let file_data = match &data.data {
            InodeKind::File(f) => f,
            _ => return Err(FsError::InvalidArgument),
        };
        
        // Get file size
        let size = self.size.load(Ordering::Relaxed);
        
        // Check if offset is beyond file size
        if off >= size {
            return Ok(0);
        }
        
        // Calculate how much to read
        let available = (size - off) as usize;
        let to_read = dst.len().min(available);
        
        // Read from chunks
        let chunk_size = file_data.chunk_size;
        let mut bytes_read = 0;
        let mut current_off = off;
        
        while bytes_read < to_read {
            let chunk_idx = (current_off / chunk_size as u64) as usize;
            let chunk_off = (current_off % chunk_size as u64) as usize;
            
            // Check if chunk exists
            if chunk_idx >= file_data.chunks.len() {
                break;
            }
            
            let chunk = &file_data.chunks[chunk_idx];
            let chunk_remaining = chunk.len() - chunk_off;
            let to_copy = (to_read - bytes_read).min(chunk_remaining);
            
            // Copy from chunk to destination
            dst[bytes_read..bytes_read + to_copy]
                .copy_from_slice(&chunk[chunk_off..chunk_off + to_copy]);
            
            bytes_read += to_copy;
            current_off += to_copy as u64;
        }
        
        Ok(bytes_read)
    }
    
    /// Write data to file at offset
    pub fn file_write_at(&self, off: u64, src: &[u8]) -> Result<usize, FsError> {
        // Lock file data
        let mut data = self.data.lock();
        let file_data = match &mut data.data {
            InodeKind::File(f) => f,
            _ => return Err(FsError::InvalidArgument),
        };
        
        let chunk_size = file_data.chunk_size;
        let mut bytes_written = 0;
        let mut current_off = off;
        
        while bytes_written < src.len() {
            let chunk_idx = (current_off / chunk_size as u64) as usize;
            let chunk_off = (current_off % chunk_size as u64) as usize;
            
            // Ensure chunk exists
            while chunk_idx >= file_data.chunks.len() {
                // Allocate new chunk filled with zeros
                let new_chunk: Arc<[u8]> = vec![0u8; chunk_size].into();
                file_data.chunks.push(new_chunk);
            }
            
            // For CoW, we need to create a new chunk if writing to an existing one
            let chunk_remaining = chunk_size - chunk_off;
            let to_copy = (src.len() - bytes_written).min(chunk_remaining);
            
            // Create a mutable copy of the chunk
            let mut chunk_vec = file_data.chunks[chunk_idx].to_vec();
            
            // Ensure chunk is large enough
            if chunk_vec.len() < chunk_off + to_copy {
                chunk_vec.resize(chunk_off + to_copy, 0);
            }
            
            // Copy data into chunk
            chunk_vec[chunk_off..chunk_off + to_copy]
                .copy_from_slice(&src[bytes_written..bytes_written + to_copy]);
            
            // Replace chunk with new version
            file_data.chunks[chunk_idx] = chunk_vec.into();
            
            bytes_written += to_copy;
            current_off += to_copy as u64;
        }
        
        // Update file size if we wrote beyond current size
        let new_size = (off + bytes_written as u64).max(self.size.load(Ordering::Relaxed));
        self.size.store(new_size, Ordering::Relaxed);
        
        // Update mtime
        data.mtime = Self::current_time();
        
        Ok(bytes_written)
    }
    
    /// Truncate file to new size
    pub fn file_truncate(&self, new_size: u64) -> Result<(), FsError> {
        // Lock file data
        let mut data = self.data.lock();
        let file_data = match &mut data.data {
            InodeKind::File(f) => f,
            _ => return Err(FsError::InvalidArgument),
        };
        
        let chunk_size = file_data.chunk_size;
        let old_size = self.size.load(Ordering::Relaxed);
        
        if new_size < old_size {
            // Shrinking file - remove excess chunks
            let new_chunk_count = ((new_size + chunk_size as u64 - 1) / chunk_size as u64) as usize;
            file_data.chunks.truncate(new_chunk_count);
            
            // If there's a partial last chunk, zero out the excess
            if new_size > 0 && new_chunk_count > 0 {
                let last_chunk_size = (new_size % chunk_size as u64) as usize;
                if last_chunk_size > 0 {
                    let last_idx = new_chunk_count - 1;
                    let mut chunk_vec = file_data.chunks[last_idx].to_vec();
                    if chunk_vec.len() > last_chunk_size {
                        chunk_vec.truncate(last_chunk_size);
                        file_data.chunks[last_idx] = chunk_vec.into();
                    }
                }
            }
        } else if new_size > old_size {
            // Growing file - add zero-filled chunks if needed
            let new_chunk_count = ((new_size + chunk_size as u64 - 1) / chunk_size as u64) as usize;
            while file_data.chunks.len() < new_chunk_count {
                let new_chunk: Arc<[u8]> = vec![0u8; chunk_size].into();
                file_data.chunks.push(new_chunk);
            }
        }
        
        // Update file size
        self.size.store(new_size, Ordering::Relaxed);
        
        // Update mtime and ctime
        let now = Self::current_time();
        data.mtime = now;
        data.ctime = now;
        
        Ok(())
    }
}
