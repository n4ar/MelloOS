//! Mello-Term - Terminal Emulator for MelloOS
//!
//! A VT/ANSI-compatible terminal emulator that provides interactive shell access
//! through the PTY subsystem.

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::vec::Vec;
use core::arch::asm;

mod allocator;
mod pty;
mod screen;
mod ansi;
mod input;
mod utf8;
mod scrollback;
mod clipboard;

use pty::PtyMaster;
use screen::ScreenBuffer;
use ansi::AnsiParser;
use scrollback::ScrollbackBuffer;
use clipboard::Clipboard;

/// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_OPEN: usize = 10;
const SYS_READ: usize = 11;
const SYS_CLOSE: usize = 12;
const SYS_IOCTL: usize = 13;

/// Raw syscall function
#[inline(always)]
unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "syscall",
        inout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

/// Write to stdout
fn sys_write(msg: &str) -> isize {
    unsafe { syscall(SYS_WRITE, 1, msg.as_ptr() as usize, msg.len()) }
}

/// Exit process
fn sys_exit(code: usize) -> ! {
    unsafe {
        syscall(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Main terminal emulator structure
pub struct MelloTerm {
    pty_master: PtyMaster,
    screen: ScreenBuffer,
    parser: AnsiParser,
    scrollback: ScrollbackBuffer,
    clipboard: Clipboard,
}

impl MelloTerm {
    /// Create a new terminal emulator instance
    pub fn new() -> Result<Self, &'static str> {
        let pty_master = PtyMaster::new()?;
        let screen = ScreenBuffer::new(25, 80);
        let parser = AnsiParser::new();
        let scrollback = ScrollbackBuffer::new();
        let clipboard = Clipboard::new();

        Ok(Self {
            pty_master,
            screen,
            parser,
            scrollback,
            clipboard,
        })
    }

    /// Copy selected text to clipboard
    pub fn copy_selection(&mut self) {
        if let Some(selection) = self.clipboard.selection() {
            // Convert screen buffer to Vec<Vec<Cell>> for clipboard
            let mut rows = alloc::vec::Vec::new();
            for row in 0..self.screen.rows {
                let mut row_cells = alloc::vec::Vec::new();
                for col in 0..self.screen.cols {
                    if let Some(cell) = self.screen.get_cell(row, col) {
                        row_cells.push(*cell);
                    }
                }
                rows.push(row_cells);
            }
            self.clipboard.copy(&rows, selection);
            sys_write("Mello-Term: Text copied to clipboard\n");
        }
    }

    /// Paste clipboard content to PTY
    pub fn paste(&mut self) -> Result<(), &'static str> {
        let content = self.clipboard.content();
        if !content.is_empty() {
            self.pty_master.write(content.as_bytes())?;
            sys_write("Mello-Term: Text pasted from clipboard\n");
        }
        Ok(())
    }

    /// Handle window resize
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), &'static str> {
        // Create new screen buffer with new dimensions
        let mut new_screen = ScreenBuffer::new(rows, cols);

        // Copy as much of the old screen as fits
        let copy_rows = rows.min(self.screen.rows);
        let copy_cols = cols.min(self.screen.cols);

        for row in 0..copy_rows {
            for col in 0..copy_cols {
                if let Some(cell) = self.screen.get_cell(row, col) {
                    new_screen.set_cell(row, col, *cell);
                }
            }
        }

        // Update cursor position (clamp to new dimensions)
        new_screen.cursor.row = self.screen.cursor.row.min(rows - 1);
        new_screen.cursor.col = self.screen.cursor.col.min(cols - 1);
        new_screen.cursor.visible = self.screen.cursor.visible;

        // Replace screen buffer
        self.screen = new_screen;

        // Notify PTY of new window size
        let winsize = pty::Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        self.pty_master.set_winsize(&winsize)?;

        sys_write("Mello-Term: Window resized\n");
        Ok(())
    }

    /// Main event loop
    pub fn run(&mut self) -> Result<(), &'static str> {
        sys_write("Mello-Term: Starting terminal emulator...\n");

        // TODO: Implement main event loop in later subtasks
        // - Read from PTY master
        // - Parse ANSI sequences
        // - Update screen buffer
        // - Handle keyboard input
        // - Render to display

        sys_write("Mello-Term: Initialized successfully\n");

        // For now, just loop
        loop {
            // Placeholder - will be implemented in subtask 7.2+
        }
    }
}

/// Entry point
#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write("Mello-Term v0.1.0\n");

    match MelloTerm::new() {
        Ok(mut term) => {
            if let Err(e) = term.run() {
                sys_write("Error: ");
                sys_write(e);
                sys_write("\n");
                sys_exit(1);
            }
        }
        Err(e) => {
            sys_write("Failed to initialize terminal: ");
            sys_write(e);
            sys_write("\n");
            sys_exit(1);
        }
    }

    sys_exit(0);
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sys_write("PANIC: ");
    if let Some(location) = info.location() {
        sys_write("at ");
        sys_write(location.file());
        sys_write(":");
        // Note: Can't easily format line number without std
    }
    sys_write(" - panic occurred\n");
    loop {}
}

/// Alloc error handler
#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    sys_write("FATAL: Memory allocation failed\n");
    loop {}
}
