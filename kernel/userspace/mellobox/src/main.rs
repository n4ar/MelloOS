//! Mellobox - Multi-call binary for MelloOS coreutils
//!
//! This binary implements common UNIX utilities in a single executable.
//! The utility to run is determined by argv[0] (the program name).

#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod syscalls;
mod error;
mod args;
mod ls;
mod cp;
mod mv;
mod rm;
mod cat;
mod grep;
mod ps;
mod kill;
mod mkdir;
mod touch;
mod echo;
mod pwd;
mod true_cmd;
mod false_cmd;

use error::{Error, Result};

/// Applet function signature
type AppletFn = fn(&'static [&'static str]) -> Result<i32>;

/// Applet registry entry
struct Applet {
    name: &'static str,
    func: AppletFn,
}

/// List of available applets
const APPLETS: &[Applet] = &[
    Applet { name: "ls", func: ls::main },
    Applet { name: "cp", func: cp::main },
    Applet { name: "mv", func: mv::main },
    Applet { name: "rm", func: rm::main },
    Applet { name: "cat", func: cat::main },
    Applet { name: "grep", func: grep::main },
    Applet { name: "ps", func: ps::main },
    Applet { name: "kill", func: kill::main },
    Applet { name: "mkdir", func: mkdir::main },
    Applet { name: "touch", func: touch::main },
    Applet { name: "echo", func: echo::main },
    Applet { name: "pwd", func: pwd::main },
    Applet { name: "true", func: true_cmd::main },
    Applet { name: "false", func: false_cmd::main },
];

/// Extract program name from argv[0]
fn get_program_name(argv0: &str) -> &str {
    // Find last '/' to get basename
    if let Some(pos) = argv0.rfind('/') {
        &argv0[pos + 1..]
    } else {
        argv0
    }
}

/// Find and execute the appropriate applet
fn dispatch(argv: &'static [&'static str]) -> Result<i32> {
    if argv.is_empty() {
        return Err(Error::InvalidArgument);
    }

    let program_name = get_program_name(argv[0]);

    // Special case: if called as "mellobox", expect applet name as first argument
    if program_name == "mellobox" {
        if argv.len() < 2 {
            print_usage();
            return Ok(0);
        }
        
        let applet_name = argv[1];
        
        // Create new argv with applet name as argv[0]
        let mut new_argv = alloc::vec::Vec::new();
        new_argv.push(applet_name);
        for i in 2..argv.len() {
            new_argv.push(argv[i]);
        }
        
        // Find and run applet
        for applet in APPLETS {
            if applet.name == applet_name {
                return (applet.func)(alloc::vec::Vec::leak(new_argv));
            }
        }
        
        error::print_usage_error("mellobox", "unknown applet");
        return Err(Error::InvalidArgument);
    }

    // Find applet by program name
    for applet in APPLETS {
        if applet.name == program_name {
            return (applet.func)(argv);
        }
    }

    // Applet not found
    error::print_usage_error("mellobox", "unknown applet");
    Err(Error::InvalidArgument)
}

/// Print usage information
fn print_usage() {
    let usage = b"Mellobox - Multi-call binary for MelloOS coreutils\n\
Usage: mellobox <applet> [args...]\n\
   or: <applet> [args...] (via symlink)\n\n\
Available applets:\n";
    
    syscalls::write(1, usage);
    
    // Print applet list
    for applet in APPLETS {
        syscalls::write(1, b"  ");
        syscalls::write(1, applet.name.as_bytes());
        syscalls::write(1, b"\n");
    }
}

/// Entry point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize heap allocator
    allocator::init();

    // Get command-line arguments from stack
    // The kernel sets up the stack with argc, argv, envp
    let argv = unsafe { get_argv() };

    // Dispatch to appropriate applet
    let exit_code = match dispatch(argv) {
        Ok(code) => code,
        Err(err) => {
            if !argv.is_empty() {
                error::print_error(argv[0], err);
            }
            err.exit_code()
        }
    };

    syscalls::exit(exit_code);
}

/// Get argv from stack
/// 
/// The kernel sets up the stack as:
/// ```
/// [argc]
/// [argv[0]]
/// [argv[1]]
/// ...
/// [argv[argc-1]]
/// [NULL]
/// [envp[0]]
/// ...
/// ```
unsafe fn get_argv() -> &'static [&'static str] {
    // Get stack pointer
    let mut rsp: usize;
    core::arch::asm!("mov {}, rsp", out(reg) rsp);
    
    // Read argc
    let argc = *(rsp as *const usize);
    rsp += 8;
    
    // Read argv pointers
    let argv_ptrs = core::slice::from_raw_parts(rsp as *const *const u8, argc);
    
    // Convert to string slices
    let mut argv = alloc::vec::Vec::with_capacity(argc);
    for &ptr in argv_ptrs {
        if ptr.is_null() {
            break;
        }
        
        // Find string length
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        
        // Create string slice
        let bytes = core::slice::from_raw_parts(ptr, len);
        if let Ok(s) = core::str::from_utf8(bytes) {
            argv.push(s);
        }
    }
    
    alloc::vec::Vec::leak(argv)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Try to print panic message
    let msg = b"PANIC: ";
    syscalls::write(2, msg);
    
    if let Some(location) = info.location() {
        let mut buf = [0u8; 256];
        let mut pos = 0;
        
        // Write file name
        for &b in location.file().as_bytes() {
            if pos >= buf.len() - 1 {
                break;
            }
            buf[pos] = b;
            pos += 1;
        }
        
        // Write ":"
        if pos < buf.len() - 1 {
            buf[pos] = b':';
            pos += 1;
        }
        
        syscalls::write(2, &buf[..pos]);
    }
    
    syscalls::write(2, b"\n");
    syscalls::exit(1);
}



