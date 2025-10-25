#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::panic::PanicInfo;

mod allocator;
mod syscalls;

use syscalls::*;

/// Print macro
macro_rules! println {
    ($fmt:expr) => {
        {
            let s = concat!($fmt, "\n");
            sys_write(1, s.as_ptr() as usize, s.len());
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut buf = [0u8; 256];
            let mut writer = BufferWriter::new(&mut buf);
            let _ = write!(writer, concat!($fmt, "\n"), $($arg)*);
            let len = writer.pos();
            sys_write(1, buf.as_ptr() as usize, len);
        }
    };
}

struct BufferWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufferWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    
    fn pos(&self) -> usize {
        self.pos
    }
}

impl<'a> core::fmt::Write for BufferWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len() - self.pos;
        let to_write = bytes.len().min(remaining);
        if to_write > 0 {
            self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
            self.pos += to_write;
        }
        Ok(())
    }
}

/// Entry point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize allocator
    allocator::init();
    
    println!("=== MelloOS Filesystem Test ===\n");
    
    // Note: Most tests will fail because filesystem syscalls are not yet implemented
    // This is expected - we're testing the infrastructure
    
    test_basic_operations();
    
    println!("\n=== Filesystem tests completed ===");
    println!("Note: Most operations returned errors because");
    println!("filesystem syscalls are not yet fully implemented.");
    println!("This is expected at this stage.");
    
    sys_exit(0);
}

fn test_basic_operations() {
    println!("Test 1: Attempting to create file /tmp/test.txt");
    let fd = sys_open("/tmp/test.txt\0".as_ptr() as usize, 0x241); // O_CREAT | O_WRONLY | O_TRUNC
    if fd < 0 {
        println!("  Result: Failed (error {})", fd);
        println!("  This is expected - filesystem not yet wired up");
    } else {
        println!("  Result: Success! FD = {}", fd);
        
        // Try to write
        println!("\nTest 2: Writing to file");
        let data = "Hello, MelloOS!\n";
        let written = sys_write(fd as usize, data.as_ptr() as usize, data.len());
        if written < 0 {
            println!("  Result: Write failed (error {})", written);
        } else {
            println!("  Result: Wrote {} bytes", written);
        }
        
        // Close the file
        println!("\nTest 3: Closing file");
        let result = sys_close(fd as usize);
        if result < 0 {
            println!("  Result: Close failed (error {})", result);
        } else {
            println!("  Result: File closed successfully");
        }
    }
    
    println!("\nTest 4: Attempting to create directory /tmp/testdir");
    let result = sys_mkdir("/tmp/testdir\0".as_ptr() as usize, 0o755);
    if result < 0 {
        println!("  Result: Failed (error {})", result);
        println!("  This is expected - mkdir not yet implemented");
    } else {
        println!("  Result: Directory created successfully");
    }
}

/// Panic handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let msg = "PANIC occurred\n";
    sys_write(1, msg.as_ptr() as usize, msg.len());
    sys_exit(1);
}
