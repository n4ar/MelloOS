//! File Descriptor Table
//!
//! This module implements per-process file descriptor tables with reference counting.
//! It handles FD allocation, flags (O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, O_CLOEXEC),
//! and thread-safe offset tracking.

// TODO: Implement file descriptor table without alloc crate
// The kernel doesn't have alloc, so we need to use static arrays instead of Vec
