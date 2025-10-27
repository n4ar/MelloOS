//! Mello-Term - Terminal Emulator for MelloOS
//!
//! A VT/ANSI-compatible terminal emulator that provides interactive shell access
//! through the PTY subsystem.

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use core::arch::asm;

mod allocator;
mod ansi;
mod clipboard;
mod input;
mod pty;
mod screen;
mod scrollback;
mod utf8;

use ansi::{AnsiParser, LineClearMode, ParseResult};
use clipboard::Clipboard;
use pty::PtyMaster;
use screen::ScreenBuffer;
use scrollback::ScrollbackBuffer;

/// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_OPEN: usize = 10;
const SYS_READ: usize = 11;
const SYS_CLOSE: usize = 12;
const SYS_IOCTL: usize = 13;
const SYS_YIELD: usize = 6;

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

/// Yield the CPU to avoid busy spinning
fn sys_yield() {
    unsafe {
        syscall(SYS_YIELD, 0, 0, 0);
    }
}

/// Main terminal emulator structure
pub struct MelloTerm {
    pty_master: PtyMaster,
    screen: ScreenBuffer,
    parser: AnsiParser,
    scrollback: ScrollbackBuffer,
    clipboard: Clipboard,
    dirty: bool,
    first_render: bool,
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
            dirty: true,
            first_render: true,
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
        self.dirty = true;
        Ok(())
    }

    /// Main event loop
    pub fn run(&mut self) -> Result<(), &'static str> {
        sys_write("Mello-Term: Starting terminal emulator...\n");
        loop {
            self.drain_pty()?;
            input::handle_keyboard_input(&self.pty_master)?;
            self.render();
            sys_yield();
        }
    }

    fn drain_pty(&mut self) -> Result<(), &'static str> {
        let mut buffer = [0u8; 512];
        loop {
            let bytes_read = self.pty_master.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            for &byte in &buffer[..bytes_read] {
                let result = self.parser.parse(byte);
                self.apply_parse_result(result);
            }
        }
        Ok(())
    }

    fn apply_parse_result(&mut self, result: ParseResult) {
        match result {
            ParseResult::None => {}
            ParseResult::Print(ch) => {
                self.screen.write_char(ch);
                self.dirty = true;
            }
            ParseResult::CursorUp(n) => {
                self.screen.cursor_up(n);
                self.dirty = true;
            }
            ParseResult::CursorDown(n) => {
                self.screen.cursor_down(n);
                self.dirty = true;
            }
            ParseResult::CursorForward(n) => {
                self.screen.cursor_forward(n);
                self.dirty = true;
            }
            ParseResult::CursorBack(n) => {
                self.screen.cursor_back(n);
                self.dirty = true;
            }
            ParseResult::CursorPosition(row, col) => {
                self.screen.move_cursor(row, col);
                self.dirty = true;
            }
            ParseResult::ClearScreen => {
                self.screen.clear();
                self.dirty = true;
            }
            ParseResult::ClearLine(mode) => {
                match mode {
                    LineClearMode::CursorToEnd => self.screen.clear_to_end_of_line(),
                    LineClearMode::CursorToStart => self.screen.clear_from_start_of_line(),
                    LineClearMode::EntireLine => self.screen.clear_line(self.screen.cursor.row),
                }
                self.dirty = true;
            }
            ParseResult::SetGraphicsMode(_params) => {
                // Attribute handling not yet implemented
            }
        }
    }

    fn render(&mut self) {
        if !self.dirty {
            return;
        }

        if self.first_render {
            sys_write("\x1b[2J");
            self.first_render = false;
        }

        sys_write("\x1b[H");
        let mut line = String::new();

        for row in 0..self.screen.rows {
            line.clear();
            for col in 0..self.screen.cols {
                if let Some(cell) = self.screen.get_cell(row, col) {
                    if cell.is_wide_continuation {
                        line.push(' ');
                    } else {
                        line.push(cell.ch);
                    }
                }
            }
            line.push('\n');
            sys_write(line.as_str());
        }

        let cursor_cmd = format!(
            "\x1b[{};{}H",
            self.screen.cursor.row + 1,
            self.screen.cursor.col + 1
        );
        sys_write(&cursor_cmd);
        self.dirty = false;
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
