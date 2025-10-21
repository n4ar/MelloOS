//! Scrollback buffer for terminal history
//!
//! Maintains a history of terminal lines for scrolling.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::screen::Cell;

/// Maximum number of lines in scrollback buffer
const MAX_SCROLLBACK_LINES: usize = 10_000;

/// A single line in the scrollback buffer
#[derive(Clone)]
pub struct ScrollbackLine {
    pub cells: Vec<Cell>,
}

impl ScrollbackLine {
    pub fn new(cells: Vec<Cell>) -> Self {
        Self { cells }
    }
}

/// Scrollback buffer managing terminal history
pub struct ScrollbackBuffer {
    /// Lines in the scrollback buffer
    lines: VecDeque<ScrollbackLine>,
    /// Maximum number of lines to keep
    max_lines: usize,
    /// Current scroll position (0 = at bottom, showing current screen)
    scroll_offset: usize,
}

impl ScrollbackBuffer {
    /// Create a new scrollback buffer
    pub fn new() -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines: MAX_SCROLLBACK_LINES,
            scroll_offset: 0,
        }
    }

    /// Add a line to the scrollback buffer
    pub fn push_line(&mut self, cells: Vec<Cell>) {
        // Add the line
        self.lines.push_back(ScrollbackLine::new(cells));

        // Evict oldest line if we exceed the limit
        if self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        let max_scroll = self.lines.len();
        self.scroll_offset = (self.scroll_offset + n).min(max_scroll);
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        if self.scroll_offset >= n {
            self.scroll_offset -= n;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.lines.len();
    }

    /// Scroll to bottom (current screen)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Get the current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Check if we're at the bottom (showing current screen)
    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    /// Get a line from the scrollback buffer
    pub fn get_line(&self, index: usize) -> Option<&ScrollbackLine> {
        self.lines.get(index)
    }

    /// Get the number of lines in the scrollback buffer
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the scrollback buffer is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clear the scrollback buffer
    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }
}
