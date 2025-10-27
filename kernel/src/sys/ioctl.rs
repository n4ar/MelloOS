//! ioctl Validation Module
//!
//! This module provides validation and security checks for ioctl operations.
//! It ensures that ioctl commands are valid, arguments are properly validated,
//! and file descriptor types match the requested operations.

use crate::sys::syscall::FdType;

/// ioctl command numbers (from Linux/POSIX)
pub const TIOCGPTN: usize = 0x80045430; // Get PTY number
pub const TCGETS: usize = 0x5401; // Get termios structure
pub const TCSETS: usize = 0x5402; // Set termios structure
pub const TIOCGWINSZ: usize = 0x5413; // Get window size
pub const TIOCSWINSZ: usize = 0x5414; // Set window size
pub const TIOCSPGRP: usize = 0x5410; // Set foreground process group
pub const TIOCGPGRP: usize = 0x540F; // Get foreground process group
pub const TIOCSCTTY: usize = 0x540E; // Make this TTY the controlling terminal

/// ioctl command categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoctlCategory {
    /// Terminal/TTY operations
    Terminal,
    /// PTY-specific operations
    Pty,
    /// File operations
    File,
    /// Unknown/unsupported
    Unknown,
}

/// ioctl command information
#[derive(Debug, Clone, Copy)]
pub struct IoctlCommand {
    /// Command number
    pub cmd: usize,
    /// Command name (for logging)
    pub name: &'static str,
    /// Category
    pub category: IoctlCategory,
    /// Whether command reads from user space
    pub reads_user: bool,
    /// Whether command writes to user space
    pub writes_user: bool,
    /// Size of argument structure (0 if not applicable)
    pub arg_size: usize,
}

impl IoctlCommand {
    /// Get command information for a given ioctl command number
    pub fn from_cmd(cmd: usize) -> Option<Self> {
        match cmd {
            TIOCGPTN => Some(IoctlCommand {
                cmd,
                name: "TIOCGPTN",
                category: IoctlCategory::Pty,
                reads_user: false,
                writes_user: true,
                arg_size: core::mem::size_of::<u32>(),
            }),
            TCGETS => Some(IoctlCommand {
                cmd,
                name: "TCGETS",
                category: IoctlCategory::Terminal,
                reads_user: false,
                writes_user: true,
                arg_size: core::mem::size_of::<crate::dev::pty::Termios>(),
            }),
            TCSETS => Some(IoctlCommand {
                cmd,
                name: "TCSETS",
                category: IoctlCategory::Terminal,
                reads_user: true,
                writes_user: false,
                arg_size: core::mem::size_of::<crate::dev::pty::Termios>(),
            }),
            TIOCGWINSZ => Some(IoctlCommand {
                cmd,
                name: "TIOCGWINSZ",
                category: IoctlCategory::Terminal,
                reads_user: false,
                writes_user: true,
                arg_size: core::mem::size_of::<crate::dev::pty::Winsize>(),
            }),
            TIOCSWINSZ => Some(IoctlCommand {
                cmd,
                name: "TIOCSWINSZ",
                category: IoctlCategory::Terminal,
                reads_user: true,
                writes_user: false,
                arg_size: core::mem::size_of::<crate::dev::pty::Winsize>(),
            }),
            TIOCSPGRP => Some(IoctlCommand {
                cmd,
                name: "TIOCSPGRP",
                category: IoctlCategory::Terminal,
                reads_user: true,
                writes_user: false,
                arg_size: core::mem::size_of::<usize>(),
            }),
            TIOCGPGRP => Some(IoctlCommand {
                cmd,
                name: "TIOCGPGRP",
                category: IoctlCategory::Terminal,
                reads_user: false,
                writes_user: true,
                arg_size: core::mem::size_of::<usize>(),
            }),
            TIOCSCTTY => Some(IoctlCommand {
                cmd,
                name: "TIOCSCTTY",
                category: IoctlCategory::Terminal,
                reads_user: false,
                writes_user: false,
                arg_size: 0, // arg is just a flag
            }),
            _ => None,
        }
    }

    /// Check if this command is valid for the given file descriptor type
    pub fn is_valid_for_fd(&self, fd_type: FdType) -> bool {
        match self.category {
            IoctlCategory::Terminal => {
                // Terminal operations valid for PTY master and slave
                matches!(fd_type, FdType::PtyMaster(_) | FdType::PtySlave(_))
            }
            IoctlCategory::Pty => {
                // PTY-specific operations only valid for PTY master
                matches!(fd_type, FdType::PtyMaster(_))
            }
            IoctlCategory::File => {
                // File operations valid for all file types
                true
            }
            IoctlCategory::Unknown => false,
        }
    }
}

/// Validate ioctl command number
///
/// # Arguments
/// * `cmd` - ioctl command number
///
/// # Returns
/// Ok(IoctlCommand) if valid, Err otherwise
pub fn validate_ioctl_cmd(cmd: usize) -> Result<IoctlCommand, &'static str> {
    IoctlCommand::from_cmd(cmd).ok_or("Invalid ioctl command")
}

/// Validate ioctl command for file descriptor type
///
/// # Arguments
/// * `cmd` - ioctl command information
/// * `fd_type` - File descriptor type
///
/// # Returns
/// Ok(()) if valid, Err otherwise
pub fn validate_ioctl_for_fd(cmd: &IoctlCommand, fd_type: FdType) -> Result<(), &'static str> {
    if cmd.is_valid_for_fd(fd_type) {
        Ok(())
    } else {
        Err("ioctl command not valid for this file descriptor type")
    }
}

/// Validate ioctl argument pointer
///
/// # Arguments
/// * `cmd` - ioctl command information
/// * `arg` - Argument pointer
///
/// # Returns
/// Ok(()) if valid, Err otherwise
pub fn validate_ioctl_arg(cmd: &IoctlCommand, arg: usize) -> Result<(), &'static str> {
    // If command doesn't use argument, any value is OK
    if cmd.arg_size == 0 {
        return Ok(());
    }

    // Validate pointer is not null
    if arg == 0 {
        return Err("ioctl argument pointer is null");
    }

    // Validate pointer is in user space
    use crate::sched::task::USER_LIMIT;
    if arg >= USER_LIMIT {
        return Err("ioctl argument pointer not in user space");
    }

    // Validate pointer + size doesn't overflow
    match arg.checked_add(cmd.arg_size) {
        Some(end) if end <= USER_LIMIT => Ok(()),
        _ => Err("ioctl argument pointer out of bounds"),
    }
}

/// Comprehensive ioctl validation
///
/// This function performs all necessary validation for an ioctl operation:
/// 1. Validates command number
/// 2. Validates command is appropriate for FD type
/// 3. Validates argument pointer
///
/// # Arguments
/// * `fd_type` - File descriptor type
/// * `cmd` - ioctl command number
/// * `arg` - Argument pointer
///
/// # Returns
/// Ok(IoctlCommand) if all validation passes, Err otherwise
pub fn validate_ioctl(
    fd_type: FdType,
    cmd: usize,
    arg: usize,
) -> Result<IoctlCommand, &'static str> {
    // Validate command number
    let ioctl_cmd = validate_ioctl_cmd(cmd)?;

    // Validate command is valid for this FD type
    validate_ioctl_for_fd(&ioctl_cmd, fd_type)?;

    // Validate argument pointer
    validate_ioctl_arg(&ioctl_cmd, arg)?;

    Ok(ioctl_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ioctl_command_lookup() {
        // Valid commands
        assert!(IoctlCommand::from_cmd(TIOCGPTN).is_some());
        assert!(IoctlCommand::from_cmd(TCGETS).is_some());
        assert!(IoctlCommand::from_cmd(TIOCSWINSZ).is_some());

        // Invalid command
        assert!(IoctlCommand::from_cmd(0xDEADBEEF).is_none());
    }

    #[test]
    fn test_ioctl_fd_type_validation() {
        let tiocgptn = IoctlCommand::from_cmd(TIOCGPTN).unwrap();
        let tcgets = IoctlCommand::from_cmd(TCGETS).unwrap();

        // TIOCGPTN only valid for PTY master
        assert!(tiocgptn.is_valid_for_fd(FdType::PtyMaster(0)));
        assert!(!tiocgptn.is_valid_for_fd(FdType::PtySlave(0)));
        assert!(!tiocgptn.is_valid_for_fd(FdType::PipeRead(0)));

        // TCGETS valid for both PTY master and slave
        assert!(tcgets.is_valid_for_fd(FdType::PtyMaster(0)));
        assert!(tcgets.is_valid_for_fd(FdType::PtySlave(0)));
        assert!(!tcgets.is_valid_for_fd(FdType::PipeRead(0)));
    }

    #[test]
    fn test_ioctl_arg_validation() {
        let tcgets = IoctlCommand::from_cmd(TCGETS).unwrap();

        // Valid pointer
        assert!(validate_ioctl_arg(&tcgets, 0x1000).is_ok());

        // Null pointer
        assert!(validate_ioctl_arg(&tcgets, 0).is_err());

        // Kernel space pointer
        assert!(validate_ioctl_arg(&tcgets, 0xFFFF_8000_0000_0000).is_err());

        // Pointer near boundary
        use crate::sched::task::USER_LIMIT;
        assert!(validate_ioctl_arg(&tcgets, USER_LIMIT - 1).is_err());
    }

    #[test]
    fn test_comprehensive_validation() {
        // Valid ioctl
        assert!(validate_ioctl(FdType::PtyMaster(0), TIOCGPTN, 0x1000).is_ok());

        // Invalid command
        assert!(validate_ioctl(FdType::PtyMaster(0), 0xDEADBEEF, 0x1000).is_err());

        // Wrong FD type
        assert!(validate_ioctl(FdType::PipeRead(0), TIOCGPTN, 0x1000).is_err());

        // Invalid argument
        assert!(validate_ioctl(FdType::PtyMaster(0), TIOCGPTN, 0).is_err());
    }
}
