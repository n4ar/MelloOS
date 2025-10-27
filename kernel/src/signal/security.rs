//! Signal Security Module
//!
//! This module provides security validation for signal operations.
//! It implements permission checks, signal handler validation, and
//! protection for critical processes.

use super::{signals, SigHandler, Signal};
use crate::mm::paging::PageTableFlags;
use crate::sched;
use crate::sched::task::{Task, USER_LIMIT};

/// Error types for signal security operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSecurityError {
    /// Permission denied (UID mismatch)
    PermissionDenied,
    /// Process not found
    ProcessNotFound,
    /// Invalid signal number
    InvalidSignal,
    /// Signal cannot be sent to this process (e.g., SIGKILL to init)
    ProtectedProcess,
    /// Signal handler address is invalid
    InvalidHandler,
    /// Session mismatch
    SessionMismatch,
}

/// Result type for signal security operations
pub type SignalSecurityResult<T> = Result<T, SignalSecurityError>;

/// Validate signal number
///
/// # Arguments
/// * `signal` - Signal number to validate
///
/// # Returns
/// Ok(()) if valid, Err otherwise
pub fn validate_signal_number(signal: Signal) -> SignalSecurityResult<()> {
    if signal == 0 || signal >= signals::MAX_SIGNAL {
        Err(SignalSecurityError::InvalidSignal)
    } else {
        Ok(())
    }
}

/// Check if sender has permission to send signal to target
///
/// Permission rules:
/// 1. Process can send signals to itself
/// 2. Process can send signals to processes in same session (for job control)
/// 3. TODO: Add UID-based permission checks when user management is implemented
///
/// # Arguments
/// * `sender` - Sending task
/// * `target` - Target task
/// * `signal` - Signal being sent
///
/// # Returns
/// Ok(()) if permission granted, Err otherwise
pub fn check_signal_permission(
    sender: &Task,
    target: &Task,
    signal: Signal,
) -> SignalSecurityResult<()> {
    // Process can signal itself
    if sender.pid == target.pid {
        return Ok(());
    }

    // For job control signals, check session membership
    if is_job_control_signal(signal) {
        if sender.sid == target.sid {
            return Ok(());
        }
        return Err(SignalSecurityError::SessionMismatch);
    }

    // Allow root to signal anyone
    if sender.creds.uid == 0 {
        return Ok(());
    }

    // Allow signaling between processes that share the same effective UID
    if sender.creds.uid == target.creds.uid {
        return Ok(());
    }

    // Allow signals within the same session (for compat with early userland)
    if sender.sid == target.sid {
        return Ok(());
    }

    // No permission
    Err(SignalSecurityError::PermissionDenied)
}

/// Check if signal is a job control signal
///
/// Job control signals can be sent across UID boundaries within the same session.
fn is_job_control_signal(signal: Signal) -> bool {
    matches!(
        signal,
        signals::SIGINT
            | signals::SIGQUIT
            | signals::SIGTSTP
            | signals::SIGTTIN
            | signals::SIGTTOU
            | signals::SIGCONT
    )
}

/// Check if process is protected from signal
///
/// Protection rules:
/// 1. Cannot send SIGKILL or SIGSTOP to PID 1 (init)
/// 2. Cannot send signals to kernel threads (PID < 100, placeholder)
///
/// # Arguments
/// * `target` - Target task
/// * `signal` - Signal being sent
///
/// # Returns
/// Ok(()) if signal can be sent, Err if process is protected
pub fn check_protected_process(target: &Task, signal: Signal) -> SignalSecurityResult<()> {
    // Protect init (PID 1) from SIGKILL and SIGSTOP
    if target.pid == 1 {
        if signal == signals::SIGKILL || signal == signals::SIGSTOP {
            return Err(SignalSecurityError::ProtectedProcess);
        }
    }

    // Kernel threads should not be signaled by userspace
    if target.creds.is_kernel_thread {
        return Err(SignalSecurityError::ProtectedProcess);
    }

    Ok(())
}

/// Validate signal handler address
///
/// Handler addresses must be:
/// 1. In user space (< USER_LIMIT)
/// 2. Not null (unless using Default or Ignore)
/// 3. Properly aligned (optional, but recommended)
///
/// # Arguments
/// * `handler` - Signal handler to validate
///
/// # Returns
/// Ok(()) if valid, Err otherwise
pub fn validate_signal_handler(handler: SigHandler) -> SignalSecurityResult<()> {
    match handler {
        SigHandler::Default | SigHandler::Ignore => {
            // Always valid
            Ok(())
        }
        SigHandler::Custom(addr) => {
            // Check if address is in user space
            if addr == 0 {
                return Err(SignalSecurityError::InvalidHandler);
            }

            if addr >= USER_LIMIT {
                return Err(SignalSecurityError::InvalidHandler);
            }

            if !is_executable_user_address(addr) {
                return Err(SignalSecurityError::InvalidHandler);
            }

            Ok(())
        }
    }
}

/// Comprehensive signal send validation
///
/// Performs all necessary security checks before sending a signal:
/// 1. Validates signal number
/// 2. Checks sender permission
/// 3. Checks if target is protected
///
/// # Arguments
/// * `sender` - Sending task
/// * `target` - Target task
/// * `signal` - Signal to send
///
/// # Returns
/// Ok(()) if all checks pass, Err otherwise
pub fn validate_signal_send(
    sender: &Task,
    target: &Task,
    signal: Signal,
) -> SignalSecurityResult<()> {
    // Validate signal number
    validate_signal_number(signal)?;

    // Check permission
    check_signal_permission(sender, target, signal)?;

    // Check if target is protected
    check_protected_process(target, signal)?;

    Ok(())
}

/// Validate signal handler registration
///
/// Performs security checks when a process registers a signal handler:
/// 1. Validates signal number
/// 2. Checks if signal can be caught
/// 3. Validates handler address
///
/// # Arguments
/// * `signal` - Signal number
/// * `handler` - Signal handler
///
/// # Returns
/// Ok(()) if valid, Err otherwise
pub fn validate_signal_handler_registration(
    signal: Signal,
    handler: SigHandler,
) -> SignalSecurityResult<()> {
    // Validate signal number
    validate_signal_number(signal)?;

    // Check if signal can be caught
    if !super::is_catchable(signal) {
        return Err(SignalSecurityError::InvalidSignal);
    }

    // Validate handler address
    validate_signal_handler(handler)?;

    Ok(())
}

/// Check if signal should be delivered to process
///
/// Checks if the signal is blocked by the process's signal mask.
///
/// # Arguments
/// * `task` - Target task
/// * `signal` - Signal to check
///
/// # Returns
/// true if signal should be delivered, false if blocked
pub fn should_deliver_signal(task: &Task, signal: Signal) -> bool {
    use core::sync::atomic::Ordering;

    // SIGKILL and SIGSTOP cannot be blocked
    if signal == signals::SIGKILL || signal == signals::SIGSTOP {
        return true;
    }

    // Check if signal is blocked
    let signal_bit = 1u64 << (signal - 1);
    let mask = task.signal_mask.load(Ordering::Relaxed);
    (mask & signal_bit) == 0
}

/// Audit log for signal operations
///
/// Logs security-relevant signal operations for debugging and auditing.
///
/// # Arguments
/// * `sender_pid` - Sender process ID
/// * `target_pid` - Target process ID
/// * `signal` - Signal number
/// * `result` - Operation result
pub fn audit_signal_send(
    sender_pid: usize,
    target_pid: usize,
    signal: Signal,
    result: SignalSecurityResult<()>,
) {
    match result {
        Ok(()) => {
            crate::serial_println!(
                "[SIGNAL_AUDIT] PID {} sent signal {} to PID {} - ALLOWED",
                sender_pid,
                signal,
                target_pid
            );
        }
        Err(err) => {
            crate::serial_println!(
                "[SIGNAL_AUDIT] PID {} attempted to send signal {} to PID {} - DENIED: {:?}",
                sender_pid,
                signal,
                target_pid,
                err
            );
        }
    }
}

fn is_executable_user_address(addr: usize) -> bool {
    if addr >= USER_LIMIT {
        return false;
    }

    if let Some(task) = sched::current_task() {
        for region_opt in &task.memory_regions[..task.region_count] {
            if let Some(region) = region_opt {
                if region.contains(addr) {
                    return (region.flags.bits() & PageTableFlags::NO_EXECUTE.bits()) == 0;
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_signal_number() {
        // Valid signals
        assert!(validate_signal_number(signals::SIGINT).is_ok());
        assert!(validate_signal_number(signals::SIGTERM).is_ok());

        // Invalid signals
        assert!(validate_signal_number(0).is_err());
        assert!(validate_signal_number(signals::MAX_SIGNAL).is_err());
        assert!(validate_signal_number(100).is_err());
    }

    #[test]
    fn test_validate_signal_handler() {
        // Valid handlers
        assert!(validate_signal_handler(SigHandler::Default).is_ok());
        assert!(validate_signal_handler(SigHandler::Ignore).is_ok());
        assert!(validate_signal_handler(SigHandler::Custom(0x1000)).is_ok());

        // Invalid handlers
        assert!(validate_signal_handler(SigHandler::Custom(0)).is_err());
        assert!(validate_signal_handler(SigHandler::Custom(USER_LIMIT)).is_err());
        assert!(validate_signal_handler(SigHandler::Custom(0xFFFF_8000_0000_0000)).is_err());
    }

    #[test]
    fn test_is_job_control_signal() {
        // Job control signals
        assert!(is_job_control_signal(signals::SIGINT));
        assert!(is_job_control_signal(signals::SIGTSTP));
        assert!(is_job_control_signal(signals::SIGCONT));

        // Non-job control signals
        assert!(!is_job_control_signal(signals::SIGKILL));
        assert!(!is_job_control_signal(signals::SIGTERM));
        assert!(!is_job_control_signal(signals::SIGHUP));
    }
}
