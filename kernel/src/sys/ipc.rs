//! IPC subsystem module
//! Provides message passing between tasks via ports

/// Maximum message size in bytes
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// IPC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// Port ID is invalid or out of range
    InvalidPort,
    /// Port's message queue is full (max 16 messages)
    QueueFull,
    /// Buffer pointer or size is invalid
    InvalidBuffer,
    /// Port does not exist
    PortNotFound,
    /// Message size exceeds 4096 bytes
    MessageTooLarge,
    /// Feature not implemented yet
    NotImplemented,
}

/// Message structure for IPC
/// 
/// Contains the raw bytes of a message. Maximum size is 4096 bytes.
/// Uses a fixed-size array to avoid heap allocation.
#[derive(Debug, Clone, Copy)]
pub struct Message {
    /// Message data (max 4096 bytes)
    pub data: [u8; MAX_MESSAGE_SIZE],
    /// Actual length of the message
    pub len: usize,
}

impl Message {
    /// Create a new empty message
    pub const fn new() -> Self {
        Self {
            data: [0; MAX_MESSAGE_SIZE],
            len: 0,
        }
    }
    
    /// Create a message from a byte slice
    /// 
    /// # Arguments
    /// * `data` - Byte slice to copy into the message
    /// 
    /// # Returns
    /// A new Message containing a copy of the data
    pub fn from_slice(data: &[u8]) -> Self {
        let mut msg = Self::new();
        let len = core::cmp::min(data.len(), MAX_MESSAGE_SIZE);
        msg.data[..len].copy_from_slice(&data[..len]);
        msg.len = len;
        msg
    }
    
    /// Get the size of the message in bytes
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if the message is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Get a slice of the message data
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
}
