//! Command history for mello-sh

use alloc::string::String;
use alloc::vec::Vec;

/// Command history
pub struct History {
    commands: Vec<String>,
    max_size: usize,
}

impl History {
    /// Create a new history
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            max_size: 1000, // Keep last 1000 commands
        }
    }

    /// Add a command to history
    pub fn add(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.trim().is_empty() {
            return;
        }

        if let Some(last) = self.commands.last() {
            if last == &command {
                return;
            }
        }

        self.commands.push(command);

        // Trim if exceeds max size
        if self.commands.len() > self.max_size {
            self.commands.remove(0);
        }
    }

    /// Get all commands
    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    /// Get command by index
    pub fn get(&self, index: usize) -> Option<&String> {
        self.commands.get(index)
    }

    /// Get number of commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
