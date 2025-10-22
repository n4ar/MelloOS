//! Mello-sh - Shell for MelloOS
//!
//! A POSIX-like shell with job control, pipelines, and I/O redirection.

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::panic::PanicInfo;

mod allocator;
mod parser;
mod executor;
mod jobs;
mod builtins;
mod history;
mod syscalls;

use jobs::JobTable;
use history::History;

/// Shell state
pub struct Shell {
    jobs: JobTable,
    history: History,
    tty_fd: Option<i32>,
    exit_requested: bool,
    env: BTreeMap<String, String>,
}

impl Shell {
    /// Create a new shell instance
    pub fn new() -> Self {
        let mut env = BTreeMap::new();
        
        // Initialize default environment variables
        env.insert(String::from("HOME"), String::from("/"));
        env.insert(String::from("PATH"), String::from("/bin"));
        env.insert(String::from("PWD"), String::from("/"));
        
        Self {
            jobs: JobTable::new(),
            history: History::new(),
            tty_fd: None,
            exit_requested: false,
            env,
        }
    }

    /// Main shell loop
    pub fn run(&mut self) -> i32 {
        // Open controlling terminal (stdin/stdout/stderr)
        // For now, we assume stdin=0, stdout=1, stderr=2 are already set up
        self.tty_fd = Some(0);

        loop {
            // Check for completed jobs
            self.jobs.check_jobs();

            // Display prompt
            self.display_prompt();

            // Read command line
            let line = match self.read_line() {
                Ok(line) => line,
                Err(_) => break, // EOF or error
            };

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Add to history
            self.history.add(line.clone());

            // Parse command
            let command = match parser::parse(&line) {
                Ok(cmd) => cmd,
                Err(e) => {
                    self.print_error(&e);
                    continue;
                }
            };

            // Execute command
            match executor::execute(self, command) {
                Ok(status) => {
                    if self.exit_requested {
                        return status;
                    }
                }
                Err(e) => {
                    self.print_error(&e);
                }
            }
        }

        0
    }

    /// Display shell prompt
    fn display_prompt(&self) {
        // Format: [user@host cwd]$
        // For now, simplified version
        syscalls::write(1, b"mello-sh$ ");
    }

    /// Read a line from stdin
    fn read_line(&self) -> Result<String, &'static str> {
        let mut buffer = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            let n = syscalls::read(0, &mut byte);
            if n <= 0 {
                if buffer.is_empty() {
                    return Err("EOF");
                }
                break;
            }

            let ch = byte[0];
            
            // Handle Ctrl-D (EOF)
            if ch == 4 && buffer.is_empty() {
                return Err("EOF");
            }

            // Handle newline
            if ch == b'\n' {
                break;
            }

            // Handle backspace
            if ch == 127 || ch == 8 {
                if !buffer.is_empty() {
                    buffer.pop();
                    // Echo backspace sequence
                    syscalls::write(1, b"\x08 \x08");
                }
                continue;
            }

            buffer.push(ch);
        }

        String::from_utf8(buffer).map_err(|_| "Invalid UTF-8")
    }

    /// Print error message
    fn print_error(&self, msg: &str) {
        syscalls::write(2, b"mello-sh: ");
        syscalls::write(2, msg.as_bytes());
        syscalls::write(2, b"\n");
    }

    /// Request shell exit
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Get job table reference
    pub fn jobs_mut(&mut self) -> &mut JobTable {
        &mut self.jobs
    }

    /// Get history reference
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Set environment variable
    pub fn set_env(&mut self, key: String, value: String) {
        self.env.insert(key, value);
    }

    /// Remove environment variable
    pub fn unset_env(&mut self, key: &str) {
        self.env.remove(key);
    }

    /// Get all environment variables
    pub fn env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    /// Get all jobs (for debugging)
    pub fn get_all_jobs(&self) -> alloc::vec::Vec<&jobs::Job> {
        self.jobs.all_jobs()
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize allocator
    allocator::init();

    // Create and run shell
    let mut shell = Shell::new();
    let exit_code = shell.run();

    // Exit
    syscalls::exit(exit_code);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to print panic message
    if let Some(location) = info.location() {
        let msg = alloc::format!("panic at {}:{}\n", location.file(), location.line());
        syscalls::write(2, msg.as_bytes());
    } else {
        syscalls::write(2, b"panic occurred\n");
    }
    
    syscalls::exit(1);
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    syscalls::write(2, b"out of memory\n");
    syscalls::exit(1);
}
