//! Signal Infrastructure
//!
//! This module implements POSIX-like signal handling for process management
//! and job control.

/// Signal number type
pub type Signal = u32;

/// Standard POSIX signals
pub mod signals {
    use super::Signal;

    /// Hangup (terminal closed)
    pub const SIGHUP: Signal = 1;
    /// Interrupt (Ctrl-C)
    pub const SIGINT: Signal = 2;
    /// Quit (Ctrl-\)
    pub const SIGQUIT: Signal = 3;
    /// Illegal instruction
    pub const SIGILL: Signal = 4;
    /// Trace/breakpoint trap
    pub const SIGTRAP: Signal = 5;
    /// Abort
    pub const SIGABRT: Signal = 6;
    /// Bus error
    pub const SIGBUS: Signal = 7;
    /// Floating point exception
    pub const SIGFPE: Signal = 8;
    /// Kill (cannot be caught)
    pub const SIGKILL: Signal = 9;
    /// User-defined signal 1
    pub const SIGUSR1: Signal = 10;
    /// Segmentation fault
    pub const SIGSEGV: Signal = 11;
    /// User-defined signal 2
    pub const SIGUSR2: Signal = 12;
    /// Broken pipe
    pub const SIGPIPE: Signal = 13;
    /// Alarm clock
    pub const SIGALRM: Signal = 14;
    /// Termination
    pub const SIGTERM: Signal = 15;
    /// Stack fault
    pub const SIGSTKFLT: Signal = 16;
    /// Child stopped or terminated
    pub const SIGCHLD: Signal = 17;
    /// Continue if stopped
    pub const SIGCONT: Signal = 18;
    /// Stop (cannot be caught)
    pub const SIGSTOP: Signal = 19;
    /// Terminal stop (Ctrl-Z)
    pub const SIGTSTP: Signal = 20;
    /// Background read from TTY
    pub const SIGTTIN: Signal = 21;
    /// Background write to TTY
    pub const SIGTTOU: Signal = 22;
    /// Urgent condition on socket
    pub const SIGURG: Signal = 23;
    /// CPU time limit exceeded
    pub const SIGXCPU: Signal = 24;
    /// File size limit exceeded
    pub const SIGXFSZ: Signal = 25;
    /// Virtual alarm clock
    pub const SIGVTALRM: Signal = 26;
    /// Profiling timer expired
    pub const SIGPROF: Signal = 27;
    /// Window size change
    pub const SIGWINCH: Signal = 28;
    /// I/O now possible
    pub const SIGIO: Signal = 29;
    /// Power failure
    pub const SIGPWR: Signal = 30;
    /// Bad system call
    pub const SIGSYS: Signal = 31;

    /// Maximum signal number
    pub const MAX_SIGNAL: Signal = 64;
}

/// Signal handler type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigHandler {
    /// Use default signal action
    Default,
    /// Ignore the signal
    Ignore,
    /// Custom user-space handler at the given address
    Custom(usize),
}

/// Signal action flags
pub mod sa_flags {
    /// Don't add signal to mask while executing handler
    pub const SA_NODEFER: u32 = 0x4000_0000;
    /// Reset handler to SIG_DFL after handling
    pub const SA_RESETHAND: u32 = 0x8000_0000;
    /// Don't receive SIGCHLD when children stop
    pub const SA_NOCLDSTOP: u32 = 0x0000_0001;
    /// Don't create zombie on child death
    pub const SA_NOCLDWAIT: u32 = 0x0000_0002;
    /// Use signal stack
    pub const SA_ONSTACK: u32 = 0x0800_0000;
    /// Restart syscalls if possible
    pub const SA_RESTART: u32 = 0x1000_0000;
}

/// Signal action structure
///
/// Describes how a process should handle a specific signal.
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    /// Signal handler
    pub handler: SigHandler,
    /// Signals to block while handler executes
    pub mask: u64,
    /// Signal action flags
    pub flags: u32,
}

impl SigAction {
    /// Create a new signal action with default handler
    pub fn default() -> Self {
        Self {
            handler: SigHandler::Default,
            mask: 0,
            flags: 0,
        }
    }

    /// Create a signal action that ignores the signal
    pub fn ignore() -> Self {
        Self {
            handler: SigHandler::Ignore,
            mask: 0,
            flags: 0,
        }
    }

    /// Create a signal action with a custom handler
    pub fn custom(handler_addr: usize, mask: u64, flags: u32) -> Self {
        Self {
            handler: SigHandler::Custom(handler_addr),
            mask,
            flags,
        }
    }
}

/// Default signal actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAction {
    /// Terminate the process
    Terminate,
    /// Ignore the signal
    Ignore,
    /// Stop the process
    Stop,
    /// Continue the process if stopped
    Continue,
    /// Terminate and dump core
    Core,
}

/// Get the default action for a signal
pub fn default_action(signal: Signal) -> DefaultAction {
    use signals::*;

    match signal {
        SIGCHLD | SIGURG | SIGWINCH => DefaultAction::Ignore,
        SIGCONT => DefaultAction::Continue,
        SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU => DefaultAction::Stop,
        SIGQUIT | SIGILL | SIGTRAP | SIGABRT | SIGBUS | SIGFPE | SIGSEGV | SIGSYS => {
            DefaultAction::Core
        }
        _ => DefaultAction::Terminate,
    }
}

/// Check if a signal can be caught or ignored
pub fn is_catchable(signal: Signal) -> bool {
    signal != signals::SIGKILL && signal != signals::SIGSTOP
}

/// Signal mask operations
pub mod sigmask {
    /// Block signals (add to mask)
    pub const SIG_BLOCK: i32 = 0;
    /// Unblock signals (remove from mask)
    pub const SIG_UNBLOCK: i32 = 1;
    /// Set signal mask
    pub const SIG_SETMASK: i32 = 2;
}
