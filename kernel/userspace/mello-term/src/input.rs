//! Keyboard input handling module
//!
//! Handles keyboard input and maps special keys to escape sequences.

use core::arch::asm;

const SYS_READ_STDIN: usize = 25;

/// Key event representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Char(char),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Backspace,
    Enter,
    Tab,
    Escape,
    Function(u8), // F1-F12
}

impl KeyEvent {
    /// Convert key event to escape sequence bytes
    pub fn to_bytes(&self) -> &[u8] {
        match self {
            KeyEvent::Char(_) => &[],
            KeyEvent::ArrowUp => b"\x1b[A",
            KeyEvent::ArrowDown => b"\x1b[B",
            KeyEvent::ArrowRight => b"\x1b[C",
            KeyEvent::ArrowLeft => b"\x1b[D",
            KeyEvent::Home => b"\x1b[H",
            KeyEvent::End => b"\x1b[F",
            KeyEvent::PageUp => b"\x1b[5~",
            KeyEvent::PageDown => b"\x1b[6~",
            KeyEvent::Insert => b"\x1b[2~",
            KeyEvent::Delete => b"\x1b[3~",
            KeyEvent::Enter => b"\r",
            KeyEvent::Backspace => b"\x7f",
            KeyEvent::Tab => b"\t",
            KeyEvent::Escape => b"\x1b",
            KeyEvent::Function(idx) => match idx {
                1 => b"\x1bOP",
                2 => b"\x1bOQ",
                3 => b"\x1bOR",
                4 => b"\x1bOS",
                5 => b"\x1b[15~",
                6 => b"\x1b[17~",
                7 => b"\x1b[18~",
                8 => b"\x1b[19~",
                9 => b"\x1b[20~",
                10 => b"\x1b[21~",
                11 => b"\x1b[23~",
                12 => b"\x1b[24~",
                _ => &[],
            },
        }
    }
}

/// Read keyboard input (platform-specific)
///
/// Note: This is a stub implementation. In a real terminal emulator,
/// this would read from stdin or a platform-specific input device.
/// For MelloOS, keyboard input would come from the kernel's keyboard driver.
pub fn read_keyboard_input() -> Option<KeyEvent> {
    let mut buf = [0u8; 1];
    let bytes_read = sys_read_stdin(&mut buf);
    if bytes_read <= 0 {
        return None;
    }

    match buf[0] {
        b'\r' | b'\n' => Some(KeyEvent::Enter),
        0x7f | 0x08 => Some(KeyEvent::Backspace),
        b'\t' => Some(KeyEvent::Tab),
        0x1b => Some(KeyEvent::Escape),
        byte => {
            if let Some(ch) = char::from_u32(byte as u32) {
                Some(KeyEvent::Char(ch))
            } else {
                None
            }
        }
    }
}

/// Handle keyboard input and write to PTY master
///
/// This function reads keyboard input and sends it to the PTY master.
/// Special keys are converted to ANSI escape sequences.
pub fn handle_keyboard_input(pty_master: &crate::pty::PtyMaster) -> Result<(), &'static str> {
    while let Some(key_event) = read_keyboard_input() {
        match key_event {
            KeyEvent::Char(ch) => {
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf);
                pty_master.write(s.as_bytes())?;
            }
            _ => {
                let bytes = key_event.to_bytes();
                if !bytes.is_empty() {
                    pty_master.write(bytes)?;
                }
            }
        }
    }
    Ok(())
}

fn sys_read_stdin(buf: &mut [u8]) -> isize {
    unsafe { syscall(SYS_READ_STDIN, buf.as_mut_ptr() as usize, buf.len(), 0) }
}

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
