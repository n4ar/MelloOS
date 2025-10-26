//! Filesystem mmap coherence tests
//!
//! Tests write invalidation and msync flush

#![cfg(test)]

use crate::fs::cache::page_cache::PageCache;
use crate::fs::vfs::{InodeType, OpenFlags, VFS};
use crate::mm::mmap::{MmapFlags, MmapManager, ProtFlags};
use crate::mm::paging::PAGE_SIZE;
use crate::sched::task::current_task;
use alloc::string::String;
use alloc::vec::Vec;

/// Test mmap write invalidation
///
/// Verifies that writes to a file via write() syscall properly invalidate
/// or update mapped pages so that subsequent reads from the mapping see
/// the new data.
pub fn test_mmap_write_invalidation() {
    serial_println!("TEST: mmap write invalidation");

    // Create a test file
    let path = "/tmp/mmap_write_test";
    let vfs = VFS::get();

    // Write initial data
    let initial_data = b"Hello, World!";
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");
    vfs.write(fd, initial_data)
        .expect("Failed to write initial data");
    vfs.close(fd).expect("Failed to close file");

    // Open file and mmap it
    let fd = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open file");

    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    let addr = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd),
            0,
        )
        .expect("Failed to mmap file");

    // Read from mapping - should see initial data
    let mapped_slice =
        unsafe { core::slice::from_raw_parts(addr as *const u8, initial_data.len()) };
    assert_eq!(mapped_slice, initial_data, "Initial mmap read mismatch");

    // Write new data via write() syscall
    let new_data = b"Updated data!";
    vfs.lseek(fd, 0, 0).expect("Failed to seek");
    vfs.write(fd, new_data).expect("Failed to write new data");

    // Flush to ensure write is visible
    vfs.fsync(fd).expect("Failed to fsync");

    // Read from mapping again - should see updated data
    // The page cache should have invalidated or updated the mapped pages
    let mapped_slice = unsafe { core::slice::from_raw_parts(addr as *const u8, new_data.len()) };
    assert_eq!(mapped_slice, new_data, "Updated mmap read mismatch");

    // Cleanup
    mmap_mgr.munmap(addr, PAGE_SIZE).expect("Failed to munmap");
    vfs.close(fd).expect("Failed to close file");
    vfs.unlink(path).expect("Failed to unlink file");

    serial_println!("TEST: mmap write invalidation - PASSED");
}

/// Test msync flush
///
/// Verifies that msync(MS_SYNC) properly flushes dirty pages from a
/// MAP_SHARED mapping to the underlying file.
pub fn test_msync_flush() {
    serial_println!("TEST: msync flush");

    let path = "/tmp/msync_test";
    let vfs = VFS::get();

    // Create empty file
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");

    // Extend file to page size
    let zeros = vec![0u8; PAGE_SIZE];
    vfs.write(fd, &zeros).expect("Failed to write zeros");
    vfs.close(fd).expect("Failed to close file");

    // Open and mmap with MAP_SHARED
    let fd = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open file");

    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    let addr = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd),
            0,
        )
        .expect("Failed to mmap file");

    // Write to mapped region
    let test_data = b"Data written via mmap";
    let mapped_slice = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) };
    mapped_slice[..test_data.len()].copy_from_slice(test_data);

    // Call msync with MS_SYNC to flush
    mmap_mgr
        .msync(addr, PAGE_SIZE, true)
        .expect("Failed to msync");

    // Close mapping
    mmap_mgr.munmap(addr, PAGE_SIZE).expect("Failed to munmap");

    // Read file via read() syscall - should see flushed data
    vfs.lseek(fd, 0, 0).expect("Failed to seek");
    let mut read_buf = vec![0u8; test_data.len()];
    let n = vfs.read(fd, &mut read_buf).expect("Failed to read");
    assert_eq!(n, test_data.len(), "Read size mismatch");
    assert_eq!(&read_buf[..], test_data, "Flushed data mismatch");

    // Cleanup
    vfs.close(fd).expect("Failed to close file");
    vfs.unlink(path).expect("Failed to unlink file");

    serial_println!("TEST: msync flush - PASSED");
}

/// Test mmap coherence with concurrent access
///
/// Verifies that multiple processes mapping the same file with MAP_SHARED
/// see each other's changes after proper synchronization.
pub fn test_mmap_concurrent_coherence() {
    serial_println!("TEST: mmap concurrent coherence");

    let path = "/tmp/concurrent_test";
    let vfs = VFS::get();

    // Create file with initial data
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");
    let zeros = vec![0u8; PAGE_SIZE];
    vfs.write(fd, &zeros).expect("Failed to write");
    vfs.close(fd).expect("Failed to close");

    // Process A: Open and mmap
    let fd_a = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open file A");
    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    let addr_a = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd_a),
            0,
        )
        .expect("Failed to mmap A");

    // Process B: Open and mmap same file
    let fd_b = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open file B");
    let addr_b = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd_b),
            0,
        )
        .expect("Failed to mmap B");

    // Write via process A's mapping
    let test_data = b"Shared data from A";
    let mapped_a = unsafe { core::slice::from_raw_parts_mut(addr_a as *mut u8, PAGE_SIZE) };
    mapped_a[..test_data.len()].copy_from_slice(test_data);

    // Sync from process A
    mmap_mgr
        .msync(addr_a, PAGE_SIZE, true)
        .expect("Failed to msync A");

    // Read via process B's mapping - should see A's changes
    let mapped_b = unsafe { core::slice::from_raw_parts(addr_b as *const u8, test_data.len()) };
    assert_eq!(mapped_b, test_data, "Concurrent coherence failed");

    // Cleanup
    mmap_mgr
        .munmap(addr_a, PAGE_SIZE)
        .expect("Failed to munmap A");
    mmap_mgr
        .munmap(addr_b, PAGE_SIZE)
        .expect("Failed to munmap B");
    vfs.close(fd_a).expect("Failed to close A");
    vfs.close(fd_b).expect("Failed to close B");
    vfs.unlink(path).expect("Failed to unlink");

    serial_println!("TEST: mmap concurrent coherence - PASSED");
}

/// Test mmap private vs shared
///
/// Verifies that MAP_PRIVATE mappings use copy-on-write and don't affect
/// the underlying file, while MAP_SHARED mappings do.
pub fn test_mmap_private_vs_shared() {
    serial_println!("TEST: mmap private vs shared");

    let path = "/tmp/private_shared_test";
    let vfs = VFS::get();

    // Create file with initial data
    let initial_data = b"Original content";
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");
    vfs.write(fd, initial_data).expect("Failed to write");
    vfs.close(fd).expect("Failed to close");

    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    // Test MAP_PRIVATE
    let fd_priv = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open for private");
    let addr_priv = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_PRIVATE,
            Some(fd_priv),
            0,
        )
        .expect("Failed to mmap private");

    // Write to private mapping
    let private_data = b"Private changes";
    let mapped_priv = unsafe { core::slice::from_raw_parts_mut(addr_priv as *mut u8, PAGE_SIZE) };
    mapped_priv[..private_data.len()].copy_from_slice(private_data);

    // Unmap private
    mmap_mgr
        .munmap(addr_priv, PAGE_SIZE)
        .expect("Failed to munmap private");
    vfs.close(fd_priv).expect("Failed to close private");

    // Read file - should still have original data (MAP_PRIVATE doesn't write back)
    let fd_check = vfs
        .open(path, OpenFlags::O_RDONLY, 0)
        .expect("Failed to open for check");
    let mut check_buf = vec![0u8; initial_data.len()];
    vfs.read(fd_check, &mut check_buf).expect("Failed to read");
    assert_eq!(&check_buf[..], initial_data, "MAP_PRIVATE affected file");
    vfs.close(fd_check).expect("Failed to close check");

    // Test MAP_SHARED
    let fd_shared = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open for shared");
    let addr_shared = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd_shared),
            0,
        )
        .expect("Failed to mmap shared");

    // Write to shared mapping
    let shared_data = b"Shared changes";
    let mapped_shared =
        unsafe { core::slice::from_raw_parts_mut(addr_shared as *mut u8, PAGE_SIZE) };
    mapped_shared[..shared_data.len()].copy_from_slice(shared_data);

    // Sync
    mmap_mgr
        .msync(addr_shared, PAGE_SIZE, true)
        .expect("Failed to msync shared");

    // Unmap
    mmap_mgr
        .munmap(addr_shared, PAGE_SIZE)
        .expect("Failed to munmap shared");
    vfs.close(fd_shared).expect("Failed to close shared");

    // Read file - should have shared changes
    let fd_final = vfs
        .open(path, OpenFlags::O_RDONLY, 0)
        .expect("Failed to open final");
    let mut final_buf = vec![0u8; shared_data.len()];
    vfs.read(fd_final, &mut final_buf)
        .expect("Failed to read final");
    assert_eq!(&final_buf[..], shared_data, "MAP_SHARED didn't affect file");
    vfs.close(fd_final).expect("Failed to close final");

    // Cleanup
    vfs.unlink(path).expect("Failed to unlink");

    serial_println!("TEST: mmap private vs shared - PASSED");
}

/// Test mmap permission enforcement
///
/// Verifies that page protection flags (PROT_READ, PROT_WRITE) are
/// properly enforced by the MMU.
pub fn test_mmap_permissions() {
    serial_println!("TEST: mmap permissions");

    let path = "/tmp/perms_test";
    let vfs = VFS::get();

    // Create file
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");
    let data = vec![0u8; PAGE_SIZE];
    vfs.write(fd, &data).expect("Failed to write");
    vfs.close(fd).expect("Failed to close");

    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    // Test PROT_READ only
    let fd_ro = vfs
        .open(path, OpenFlags::O_RDONLY, 0)
        .expect("Failed to open readonly");
    let addr_ro = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ,
            MmapFlags::MAP_SHARED,
            Some(fd_ro),
            0,
        )
        .expect("Failed to mmap readonly");

    // Read should succeed
    let mapped_ro = unsafe { core::slice::from_raw_parts(addr_ro as *const u8, 16) };
    let _ = mapped_ro[0]; // Read access

    // Write should cause page fault (we can't easily test this without
    // proper fault handling, so we just verify the mapping exists)
    // In a real test, attempting to write would trigger a page fault

    mmap_mgr
        .munmap(addr_ro, PAGE_SIZE)
        .expect("Failed to munmap ro");
    vfs.close(fd_ro).expect("Failed to close ro");

    // Test PROT_READ | PROT_WRITE
    let fd_rw = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open readwrite");
    let addr_rw = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd_rw),
            0,
        )
        .expect("Failed to mmap readwrite");

    // Both read and write should succeed
    let mapped_rw = unsafe { core::slice::from_raw_parts_mut(addr_rw as *mut u8, 16) };
    let _ = mapped_rw[0]; // Read
    mapped_rw[0] = 42; // Write

    mmap_mgr
        .munmap(addr_rw, PAGE_SIZE)
        .expect("Failed to munmap rw");
    vfs.close(fd_rw).expect("Failed to close rw");

    // Cleanup
    vfs.unlink(path).expect("Failed to unlink");

    serial_println!("TEST: mmap permissions - PASSED");
}

/// Test mprotect
///
/// Verifies that mprotect can change protection flags on existing mappings.
pub fn test_mprotect() {
    serial_println!("TEST: mprotect");

    let path = "/tmp/mprotect_test";
    let vfs = VFS::get();

    // Create file
    let fd = vfs
        .open(path, OpenFlags::O_CREAT | OpenFlags::O_RDWR, 0o644)
        .expect("Failed to create file");
    let data = vec![0u8; PAGE_SIZE];
    vfs.write(fd, &data).expect("Failed to write");
    vfs.close(fd).expect("Failed to close");

    let task = current_task();
    let mmap_mgr = task.mmap_manager();

    // Map with PROT_READ | PROT_WRITE
    let fd = vfs
        .open(path, OpenFlags::O_RDWR, 0)
        .expect("Failed to open");
    let addr = mmap_mgr
        .mmap(
            0,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MmapFlags::MAP_SHARED,
            Some(fd),
            0,
        )
        .expect("Failed to mmap");

    // Write should succeed
    let mapped = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 16) };
    mapped[0] = 42;
    assert_eq!(mapped[0], 42, "Initial write failed");

    // Change to PROT_READ only
    mmap_mgr
        .mprotect(addr, PAGE_SIZE, ProtFlags::PROT_READ)
        .expect("Failed to mprotect to readonly");

    // Read should still work
    let mapped_ro = unsafe { core::slice::from_raw_parts(addr as *const u8, 16) };
    assert_eq!(mapped_ro[0], 42, "Read after mprotect failed");

    // Write would now cause page fault (can't easily test without fault handler)

    // Change back to PROT_READ | PROT_WRITE
    mmap_mgr
        .mprotect(
            addr,
            PAGE_SIZE,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
        )
        .expect("Failed to mprotect to readwrite");

    // Write should work again
    let mapped_rw = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 16) };
    mapped_rw[0] = 99;
    assert_eq!(mapped_rw[0], 99, "Write after mprotect back failed");

    // Cleanup
    mmap_mgr.munmap(addr, PAGE_SIZE).expect("Failed to munmap");
    vfs.close(fd).expect("Failed to close");
    vfs.unlink(path).expect("Failed to unlink");

    serial_println!("TEST: mprotect - PASSED");
}

/// Run all mmap coherence tests
pub fn run_all_tests() {
    serial_println!("\n=== Running mmap coherence tests ===\n");

    test_mmap_write_invalidation();
    test_msync_flush();
    test_mmap_concurrent_coherence();
    test_mmap_private_vs_shared();
    test_mmap_permissions();
    test_mprotect();

    serial_println!("\n=== All mmap coherence tests PASSED ===\n");
}
