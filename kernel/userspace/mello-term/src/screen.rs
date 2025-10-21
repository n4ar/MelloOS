//! Screen buffer and rendering module
//!
//! Manages the terminal screen buffer and cursor state.

use alloc::vec::Vec;

/// Color enumeration for terminal cells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

/// Cell attributes (bold, underline, etc.)
#[derive(Debug, Clone, Copy, Default)]
pub struct Attributes {
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
    pub blink: bool,
}

/// A single cell in the screen buffer
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
    /// True if this cell is the second half of a wide character
    pub is_wide_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::White,
            bg: Color::Black,
            attrs: Attributes::default(),
            is_wide_continuation: false,
        }
    }
}

/// Cursor position and visibility
#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            row: 0,
            col: 0,
            visible: true,
        }
    }
}

/// Screen buffer managing the terminal display
pub struct ScreenBuffer {
    pub rows: u16,
    pub cols: u16,
    cells: Vec<Cell>,
    pub cursor: Cursor,
}

impl ScreenBuffer {
    /// Create a new screen buffer with specified dimensions
    pub fn new(rows: u16, cols: u16) -> Self {
        let size = (rows as usize) * (cols as usize);
        let cells = alloc::vec![Cell::default(); size];

        Self {
            rows,
            cols,
            cells,
            cursor: Cursor::default(),
        }
    }

    /// Get a cell at the specified position
    pub fn get_cell(&self, row: u16, col: u16) -> Option<&Cell> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        let idx = (row as usize) * (self.cols as usize) + (col as usize);
        self.cells.get(idx)
    }

    /// Set a cell at the specified position
    pub fn set_cell(&mut self, row: u16, col: u16, cell: Cell) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        let idx = (row as usize) * (self.cols as usize) + (col as usize);
        if let Some(c) = self.cells.get_mut(idx) {
            *c = cell;
        }
    }

    /// Write a character at the current cursor position
    pub fn write_char(&mut self, ch: char) {
        match ch {
            '\n' => {
                // Newline: move to start of next line
                self.cursor.col = 0;
                self.cursor.row += 1;
                if self.cursor.row >= self.rows {
                    self.scroll_up();
                    self.cursor.row = self.rows - 1;
                }
            }
            '\r' => {
                // Carriage return: move to start of current line
                self.cursor.col = 0;
            }
            '\t' => {
                // Tab: move to next tab stop (every 8 columns)
                let next_tab = ((self.cursor.col / 8) + 1) * 8;
                self.cursor.col = next_tab.min(self.cols - 1);
            }
            '\x08' => {
                // Backspace: move cursor back one position
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            _ => {
                // Regular character: write at cursor position
                if ch.is_control() {
                    // Skip other control characters
                    return;
                }

                // Get character width
                let width = crate::utf8::char_width(ch);
                
                // Skip combining characters (width 0) - they should be handled specially
                if width == 0 && !crate::utf8::is_combining(ch) {
                    return;
                }

                // Write character at cursor position
                let cell = Cell {
                    ch,
                    fg: Color::White,
                    bg: Color::Black,
                    attrs: Attributes::default(),
                    is_wide_continuation: false,
                };
                self.set_cell(self.cursor.row, self.cursor.col, cell);

                // For wide characters (width 2), mark the next cell as continuation
                if width == 2 && self.cursor.col + 1 < self.cols {
                    let continuation = Cell {
                        ch: ' ',
                        fg: Color::White,
                        bg: Color::Black,
                        attrs: Attributes::default(),
                        is_wide_continuation: true,
                    };
                    self.set_cell(self.cursor.row, self.cursor.col + 1, continuation);
                }

                // Advance cursor by character width
                self.cursor.col += width as u16;

                // Wrap to next line if needed
                if self.cursor.col >= self.cols {
                    self.cursor.col = 0;
                    self.cursor.row += 1;
                    if self.cursor.row >= self.rows {
                        self.scroll_up();
                        self.cursor.row = self.rows - 1;
                    }
                }
            }
        }
    }

    /// Scroll the screen buffer up by one line
    pub fn scroll_up(&mut self) {
        // Move all lines up by one
        let cols = self.cols as usize;
        let rows = self.rows as usize;

        // Copy each line to the previous line
        for row in 1..rows {
            for col in 0..cols {
                let src_idx = row * cols + col;
                let dst_idx = (row - 1) * cols + col;
                if let Some(cell) = self.cells.get(src_idx).copied() {
                    if let Some(dst) = self.cells.get_mut(dst_idx) {
                        *dst = cell;
                    }
                }
            }
        }

        // Clear the last line
        let last_row = rows - 1;
        for col in 0..cols {
            let idx = last_row * cols + col;
            if let Some(cell) = self.cells.get_mut(idx) {
                *cell = Cell::default();
            }
        }
    }

    /// Clear the entire screen
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
        self.cursor = Cursor::default();
    }

    /// Move cursor to specified position
    pub fn move_cursor(&mut self, row: u16, col: u16) {
        self.cursor.row = row.min(self.rows - 1);
        self.cursor.col = col.min(self.cols - 1);
    }

    /// Move cursor up by n rows
    pub fn cursor_up(&mut self, n: u16) {
        if self.cursor.row >= n {
            self.cursor.row -= n;
        } else {
            self.cursor.row = 0;
        }
    }

    /// Move cursor down by n rows
    pub fn cursor_down(&mut self, n: u16) {
        self.cursor.row = (self.cursor.row + n).min(self.rows - 1);
    }

    /// Move cursor forward by n columns
    pub fn cursor_forward(&mut self, n: u16) {
        self.cursor.col = (self.cursor.col + n).min(self.cols - 1);
    }

    /// Move cursor back by n columns
    pub fn cursor_back(&mut self, n: u16) {
        if self.cursor.col >= n {
            self.cursor.col -= n;
        } else {
            self.cursor.col = 0;
        }
    }

    /// Clear from cursor to end of screen
    pub fn clear_to_end_of_screen(&mut self) {
        let start_idx = (self.cursor.row as usize) * (self.cols as usize) + (self.cursor.col as usize);
        for i in start_idx..self.cells.len() {
            self.cells[i] = Cell::default();
        }
    }

    /// Clear from cursor to end of line
    pub fn clear_to_end_of_line(&mut self) {
        let row_start = (self.cursor.row as usize) * (self.cols as usize);
        let start_idx = row_start + (self.cursor.col as usize);
        let end_idx = row_start + (self.cols as usize);
        for i in start_idx..end_idx {
            if let Some(cell) = self.cells.get_mut(i) {
                *cell = Cell::default();
            }
        }
    }

    /// Clear entire line
    pub fn clear_line(&mut self, row: u16) {
        if row >= self.rows {
            return;
        }
        let row_start = (row as usize) * (self.cols as usize);
        let row_end = row_start + (self.cols as usize);
        for i in row_start..row_end {
            if let Some(cell) = self.cells.get_mut(i) {
                *cell = Cell::default();
            }
        }
    }
}
