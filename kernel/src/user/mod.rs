/// User-mode support module
/// 
/// This module provides functionality for user-mode execution including:
/// - ELF binary loading and parsing
/// - Process management
/// - User-kernel memory management

pub mod elf;

pub use elf::*;