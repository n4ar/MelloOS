//! ANSI/VT escape sequence parser
//!
//! Implements a state machine for parsing ANSI escape sequences.

use alloc::vec::Vec;

/// Parser state for ANSI escape sequences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    Normal,
    Escape,
    Csi,
}

/// Modes for clearing parts of the current line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClearMode {
    CursorToEnd,
    CursorToStart,
    EntireLine,
}

/// ANSI escape sequence parser
pub struct AnsiParser {
    state: ParserState,
    params: Vec<u32>,
    current_param: u32,
    has_param: bool,
}

impl AnsiParser {
    /// Create a new ANSI parser
    pub fn new() -> Self {
        Self {
            state: ParserState::Normal,
            params: Vec::new(),
            current_param: 0,
            has_param: false,
        }
    }

    /// Parse a byte and return any resulting action
    pub fn parse(&mut self, byte: u8) -> ParseResult {
        match self.state {
            ParserState::Normal => self.parse_normal(byte),
            ParserState::Escape => self.parse_escape(byte),
            ParserState::Csi => self.parse_csi(byte),
        }
    }

    /// Parse a byte in normal state
    fn parse_normal(&mut self, byte: u8) -> ParseResult {
        match byte {
            0x1B => {
                // ESC character - enter escape state
                self.state = ParserState::Escape;
                ParseResult::None
            }
            _ => {
                // Regular character - print it
                if let Some(ch) = char::from_u32(byte as u32) {
                    ParseResult::Print(ch)
                } else {
                    ParseResult::None
                }
            }
        }
    }

    /// Parse a byte in escape state
    fn parse_escape(&mut self, byte: u8) -> ParseResult {
        match byte {
            b'[' => {
                // CSI sequence - enter CSI state
                self.state = ParserState::Csi;
                self.params.clear();
                self.current_param = 0;
                self.has_param = false;
                ParseResult::None
            }
            _ => {
                // Unknown escape sequence - reset to normal
                self.reset();
                ParseResult::None
            }
        }
    }

    /// Parse a byte in CSI state
    fn parse_csi(&mut self, byte: u8) -> ParseResult {
        match byte {
            b'0'..=b'9' => {
                // Digit - accumulate parameter
                self.current_param = self.current_param * 10 + (byte - b'0') as u32;
                self.has_param = true;
                ParseResult::None
            }
            b';' => {
                // Parameter separator
                if self.has_param {
                    self.params.push(self.current_param);
                } else {
                    self.params.push(0);
                }
                self.current_param = 0;
                self.has_param = false;
                ParseResult::None
            }
            b'A' => {
                // Cursor up
                let n = if self.has_param {
                    self.current_param
                } else {
                    1
                };
                self.reset();
                ParseResult::CursorUp(n as u16)
            }
            b'B' => {
                // Cursor down
                let n = if self.has_param {
                    self.current_param
                } else {
                    1
                };
                self.reset();
                ParseResult::CursorDown(n as u16)
            }
            b'C' => {
                // Cursor forward
                let n = if self.has_param {
                    self.current_param
                } else {
                    1
                };
                self.reset();
                ParseResult::CursorForward(n as u16)
            }
            b'D' => {
                // Cursor back
                let n = if self.has_param {
                    self.current_param
                } else {
                    1
                };
                self.reset();
                ParseResult::CursorBack(n as u16)
            }
            b'H' | b'f' => {
                // Cursor position (row;col)
                if self.has_param {
                    self.params.push(self.current_param);
                }

                let row = self.params.get(0).copied().unwrap_or(1).saturating_sub(1);
                let col = self.params.get(1).copied().unwrap_or(1).saturating_sub(1);

                self.reset();
                ParseResult::CursorPosition(row as u16, col as u16)
            }
            b'J' => {
                // Clear screen
                let mode = if self.has_param {
                    self.current_param
                } else {
                    0
                };

                self.reset();

                // Mode 2 = clear entire screen
                if mode == 2 {
                    ParseResult::ClearScreen
                } else {
                    // Other modes not yet implemented
                    ParseResult::None
                }
            }
            b'K' => {
                // Clear line
                let mode = if self.has_param {
                    self.current_param
                } else {
                    0
                };

                self.reset();
                let clear_mode = match mode {
                    1 => LineClearMode::CursorToStart,
                    2 => LineClearMode::EntireLine,
                    _ => LineClearMode::CursorToEnd,
                };
                ParseResult::ClearLine(clear_mode)
            }
            b'm' => {
                // SGR - Set Graphics Rendition (colors/attributes)
                if self.has_param {
                    self.params.push(self.current_param);
                }

                // If no parameters, default to 0 (reset)
                if self.params.is_empty() {
                    self.params.push(0);
                }

                let params = self.params.clone();
                self.reset();
                ParseResult::SetGraphicsMode(params)
            }
            _ => {
                // Unknown CSI sequence - reset to normal
                self.reset();
                ParseResult::None
            }
        }
    }

    /// Reset parser state
    pub fn reset(&mut self) {
        self.state = ParserState::Normal;
        self.params.clear();
        self.current_param = 0;
        self.has_param = false;
    }
}

/// Result of parsing a byte
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    None,
    Print(char),
    CursorUp(u16),
    CursorDown(u16),
    CursorForward(u16),
    CursorBack(u16),
    CursorPosition(u16, u16),
    ClearScreen,
    ClearLine(LineClearMode),
    SetGraphicsMode(Vec<u32>),
}
