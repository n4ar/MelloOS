//! Clipboard support for copy/paste operations
//!
//! Provides text selection and clipboard management.

use crate::screen::Cell;
use alloc::string::String;
use alloc::vec::Vec;

/// Text selection region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start_row: u16,
    pub start_col: u16,
    pub end_row: u16,
    pub end_col: u16,
}

impl Selection {
    /// Create a new selection
    pub fn new(start_row: u16, start_col: u16, end_row: u16, end_col: u16) -> Self {
        Self {
            start_row,
            start_col,
            end_row,
            end_col,
        }
    }

    /// Check if a cell is within the selection
    pub fn contains(&self, row: u16, col: u16) -> bool {
        // Normalize selection (ensure start is before end)
        let (start_row, start_col, end_row, end_col) = if self.start_row < self.end_row
            || (self.start_row == self.end_row && self.start_col <= self.end_col)
        {
            (self.start_row, self.start_col, self.end_row, self.end_col)
        } else {
            (self.end_row, self.end_col, self.start_row, self.start_col)
        };

        // Check if cell is in selection
        if row < start_row || row > end_row {
            return false;
        }

        if row == start_row && row == end_row {
            col >= start_col && col <= end_col
        } else if row == start_row {
            col >= start_col
        } else if row == end_row {
            col <= end_col
        } else {
            true
        }
    }
}

/// Clipboard manager
pub struct Clipboard {
    /// Current clipboard content
    content: String,
    /// Current selection (if any)
    selection: Option<Selection>,
}

impl Clipboard {
    /// Create a new clipboard
    pub fn new() -> Self {
        Self {
            content: String::new(),
            selection: None,
        }
    }

    /// Set the current selection
    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.selection = selection;
    }

    /// Get the current selection
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// Copy selected text to clipboard
    pub fn copy(&mut self, cells: &[Vec<Cell>], selection: Selection) {
        let mut text = String::new();

        // Normalize selection
        let (start_row, start_col, end_row, end_col) = if selection.start_row < selection.end_row
            || (selection.start_row == selection.end_row
                && selection.start_col <= selection.end_col)
        {
            (
                selection.start_row,
                selection.start_col,
                selection.end_row,
                selection.end_col,
            )
        } else {
            (
                selection.end_row,
                selection.end_col,
                selection.start_row,
                selection.start_col,
            )
        };

        // Extract text from selection
        for row in start_row..=end_row {
            if let Some(row_cells) = cells.get(row as usize) {
                let start = if row == start_row {
                    start_col as usize
                } else {
                    0
                };
                let end = if row == end_row {
                    (end_col as usize + 1).min(row_cells.len())
                } else {
                    row_cells.len()
                };

                for col in start..end {
                    if let Some(cell) = row_cells.get(col) {
                        if !cell.is_wide_continuation {
                            text.push(cell.ch);
                        }
                    }
                }

                // Add newline between rows (except for last row)
                if row < end_row {
                    text.push('\n');
                }
            }
        }

        self.content = text;
    }

    /// Get clipboard content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Paste clipboard content
    pub fn paste(&self) -> &str {
        &self.content
    }

    /// Clear clipboard
    pub fn clear(&mut self) {
        self.content.clear();
        self.selection = None;
    }
}
