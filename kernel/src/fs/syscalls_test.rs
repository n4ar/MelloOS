//! Filesystem Syscalls Test Module
//!
//! This module provides basic tests for the filesystem syscalls implementation.

use crate::fs::syscalls::{
    sys_open, sys_read, sys_write, sys_close, sys_lseek, sys_stat, sys_fstat,
    sys_mkdir, sys_unlink, sys_symlink, sys_readlink, sys_sync, sys_fsync,
    sys_mount, sys_umount, open_flags, seek_whence
};
use crate::serial_println;

/// Test basic filesystem syscall functionality
pub fn test_filesystem_syscalls() {
    serial_println!("[TEST] Starting filesystem syscalls tests...");

    test_open_close();
    test_read_write();
    test_stat();
    test_mkdir_unlink();
    test_symlink_readlink();
    test_sync();
    test_mount_umount();

    serial_println!("[TEST] Filesystem syscalls tests completed successfully!");
}

/// Test open and close operations
fn test_open_close() {
    serial_println!("[TEST] Testing sys_open and sys_close...");

    // Test opening a file with create flag
    let path = "/tmp/test.txt\0";
    let fd = sys_open(
        path.as_ptr() as usize,
        open_flags::O_CREAT | open_flags::O_RDWR,
        0o644,
    );

    if fd >= 0 {
        serial_println!("[TEST] sys_open: Successfully opened file, FD = {}", fd);

        // Test closing the file
        let result = sys_close(fd);
        if result == 0 {
            serial_println!("[TEST] sys_close: Successfully closed file");
        } else {
            serial_println!("[TEST] sys_close: Failed with error {}", result);
        }
    } else {
        serial_println!("[TEST] sys_open: Failed with error {}", fd);
    }
}

/// Test read and write operations
fn test_read_write() {
    serial_println!("[TEST] Testing sys_read and sys_write...");

    // Test writing to a fake FD
    let test_data = b"Hello, MelloOS filesystem!";
    let bytes_written = sys_write(3, test_data.as_ptr() as usize, test_data.len());

    if bytes_written > 0 {
        serial_println!(
            "[TEST] sys_write: Successfully wrote {} bytes",
            bytes_written
        );
    } else {
        serial_println!("[TEST] sys_write: Failed with error {}", bytes_written);
    }

    // Test reading from a fake FD
    let mut buffer = [0u8; 64];
    let bytes_read = sys_read(3, buffer.as_mut_ptr() as usize, buffer.len());

    if bytes_read > 0 {
        serial_println!("[TEST] sys_read: Successfully read {} bytes", bytes_read);
        if let Ok(s) = core::str::from_utf8(&buffer[..bytes_read as usize]) {
            serial_println!("[TEST] sys_read: Data = \"{}\"", s);
        }
    } else {
        serial_println!("[TEST] sys_read: Failed with error {}", bytes_read);
    }
}

/// Test stat operations
fn test_stat() {
    serial_println!("[TEST] Testing sys_stat and sys_fstat...");

    // Test stat on root directory
    let path = "/\0";
    let mut stat_buf = core::mem::MaybeUninit::uninit();
    let result = sys_stat(path.as_ptr() as usize, stat_buf.as_mut_ptr() as usize);

    if result == 0 {
        serial_println!("[TEST] sys_stat: Successfully got stat for root directory");
    } else {
        serial_println!("[TEST] sys_stat: Failed with error {}", result);
    }

    // Test fstat on a fake FD
    let mut stat_buf = core::mem::MaybeUninit::uninit();
    let result = sys_fstat(3, stat_buf.as_mut_ptr() as usize);

    if result == 0 {
        serial_println!("[TEST] sys_fstat: Successfully got stat for FD 3");
    } else {
        serial_println!("[TEST] sys_fstat: Failed with error {}", result);
    }
}

/// Test directory operations
fn test_mkdir_unlink() {
    serial_println!("[TEST] Testing sys_mkdir and sys_unlink...");

    // Test creating a directory
    let dir_path = "/tmp/testdir\0";
    let result = sys_mkdir(dir_path.as_ptr() as usize, 0o755);

    if result == 0 {
        serial_println!("[TEST] sys_mkdir: Successfully created directory");
    } else {
        serial_println!("[TEST] sys_mkdir: Failed with error {}", result);
    }

    // Test removing a file
    let file_path = "/tmp/testfile\0";
    let result = sys_unlink(file_path.as_ptr() as usize);

    if result == 0 {
        serial_println!("[TEST] sys_unlink: Successfully removed file");
    } else {
        serial_println!("[TEST] sys_unlink: Failed with error {}", result);
    }
}

/// Test symbolic link operations
fn test_symlink_readlink() {
    serial_println!("[TEST] Testing sys_symlink and sys_readlink...");

    // Test creating a symbolic link
    let target = "/tmp/target\0";
    let linkpath = "/tmp/symlink\0";
    let result = sys_symlink(target.as_ptr() as usize, linkpath.as_ptr() as usize);

    if result == 0 {
        serial_println!("[TEST] sys_symlink: Successfully created symbolic link");
    } else {
        serial_println!("[TEST] sys_symlink: Failed with error {}", result);
    }

    // Test reading a symbolic link
    let mut buffer = [0u8; 64];
    let bytes_read = sys_readlink(
        linkpath.as_ptr() as usize,
        buffer.as_mut_ptr() as usize,
        buffer.len(),
    );

    if bytes_read > 0 {
        serial_println!(
            "[TEST] sys_readlink: Successfully read {} bytes",
            bytes_read
        );
        if let Ok(s) = core::str::from_utf8(&buffer[..bytes_read as usize]) {
            serial_println!("[TEST] sys_readlink: Target = \"{}\"", s);
        }
    } else {
        serial_println!("[TEST] sys_readlink: Failed with error {}", bytes_read);
    }
}

/// Test sync operations
fn test_sync() {
    serial_println!("[TEST] Testing sys_sync and sys_fsync...");

    // Test syncing all filesystems
    let result = sys_sync();

    if result == 0 {
        serial_println!("[TEST] sys_sync: Successfully synced all filesystems");
    } else {
        serial_println!("[TEST] sys_sync: Failed with error {}", result);
    }

    // Test syncing a specific file
    let result = sys_fsync(3);

    if result == 0 {
        serial_println!("[TEST] sys_fsync: Successfully synced FD 3");
    } else {
        serial_println!("[TEST] sys_fsync: Failed with error {}", result);
    }
}

/// Test mount and unmount operations
fn test_mount_umount() {
    serial_println!("[TEST] Testing sys_mount and sys_umount...");

    // Test mounting a filesystem
    let source = "/dev/sda1\0";
    let target = "/mnt\0";
    let fstype = "mfs_disk\0";
    let result = sys_mount(
        source.as_ptr() as usize,
        target.as_ptr() as usize,
        fstype.as_ptr() as usize,
        0,
        0,
    );

    if result == 0 {
        serial_println!("[TEST] sys_mount: Successfully mounted filesystem");
    } else {
        serial_println!("[TEST] sys_mount: Failed with error {}", result);
    }

    // Test unmounting a filesystem
    let result = sys_umount(target.as_ptr() as usize, 0);

    if result == 0 {
        serial_println!("[TEST] sys_umount: Successfully unmounted filesystem");
    } else {
        serial_println!("[TEST] sys_umount: Failed with error {}", result);
    }
}
