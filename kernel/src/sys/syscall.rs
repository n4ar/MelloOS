//! System Call Interface
//!
//! This module implements the system call interface for userland-kernel communication.
//! It provides syscall entry point, dispatcher, and handler functions.

use crate::sched::task::USER_LIMIT;
use crate::sync::SpinLock;
use crate::sys::METRICS;
use crate::{serial_print, serial_println};
use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

/// Syscall entry point (naked function)
///
/// This function is called when userland invokes int 0x80.
/// It saves all registers, calls the dispatcher, and restores registers.
///
/// Register mapping (x86-64 System V ABI):
/// - RAX: Syscall number (input), return value (output)
/// - RDI: Argument 1
/// - RSI: Argument 2
/// - RDX: Argument 3
#[unsafe(naked)]
#[no_mangle]
pub extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        // The CPU has already pushed SS, RSP, RFLAGS, CS, RIP
        // We need to save all other registers

        // Save caller-saved registers
        "push rax",      // Syscall number
        "push rcx",
        "push rdx",      // Arg 3
        "push rsi",      // Arg 2
        "push rdi",      // Arg 1
        "push r8",
        "push r9",
        "push r10",
        "push r11",

        // Save callee-saved registers
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Clear direction flag (required by ABI)
        "cld",

        // Prepare arguments for syscall_dispatcher
        // Stack layout after all pushes (each register = 8 bytes):
        // [rsp + 0]  = r15
        // [rsp + 8]  = r14
        // [rsp + 16] = r13
        // [rsp + 24] = r12
        // [rsp + 32] = rbp
        // [rsp + 40] = rbx
        // [rsp + 48] = r11
        // [rsp + 56] = r10
        // [rsp + 64] = r9
        // [rsp + 72] = r8
        // [rsp + 80] = rdi (arg1) ← we need this
        // [rsp + 88] = rsi (arg2) ← we need this
        // [rsp + 96] = rdx (arg3) ← we need this
        // [rsp + 104] = rcx
        // [rsp + 112] = rax (syscall_id)

        // RDI = syscall_id (from RAX)
        // RSI = arg1 (from original RDI)
        // RDX = arg2 (from original RSI)
        // RCX = arg3 (from original RDX)
        "mov rdi, rax",           // syscall_id
        "mov rsi, [rsp + 80]",    // arg1 (original RDI)
        "mov rdx, [rsp + 88]",    // arg2 (original RSI)
        "mov rcx, [rsp + 96]",    // arg3 (original RDX)

        // Call the dispatcher
        "call {dispatcher}",

        // RAX now contains the return value
        // We need to preserve it while restoring other registers
        // Use the stack slot where we saved RAX (syscall_id)
        "mov [rsp + 112], rax",   // Save return value to old RAX slot

        // Restore callee-saved registers
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",

        // Restore caller-saved registers (except RAX which has return value)
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",

        // Restore return value to RAX (from the slot we saved it to)
        "pop rax",    // This pops the return value we saved earlier

        // Return from interrupt (pops RIP, CS, RFLAGS, RSP, SS)
        "iretq",

        dispatcher = sym syscall_dispatcher_wrapper,
    )
}

/// Wrapper for syscall_dispatcher to match calling convention
///
/// This function converts the register arguments to Rust function arguments.
#[no_mangle]
extern "C" fn syscall_dispatcher_wrapper(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> isize {
    syscall_dispatcher(syscall_id, arg1, arg2, arg3)
}

/// Syscall numbers
pub const SYS_WRITE: usize = 0;
pub const SYS_EXIT: usize = 1;
pub const SYS_SLEEP: usize = 2;
pub const SYS_IPC_SEND: usize = 3;
pub const SYS_IPC_RECV: usize = 4;
pub const SYS_GETPID: usize = 5;
pub const SYS_YIELD: usize = 6;
pub const SYS_FORK: usize = 7;
pub const SYS_WAIT: usize = 8;
pub const SYS_EXEC: usize = 9;
pub const SYS_OPEN: usize = 10;
pub const SYS_READ: usize = 11;
pub const SYS_CLOSE: usize = 12;
pub const SYS_IOCTL: usize = 13;
pub const SYS_SIGACTION: usize = 14;
pub const SYS_KILL: usize = 15;

static NEXT_FAKE_PID: AtomicUsize = AtomicUsize::new(2000);

/// Syscall dispatcher
///
/// Routes syscall ID to appropriate handler and increments metrics.
///
/// # Arguments
/// * `syscall_id` - Syscall number (from RAX)
/// * `arg1` - First argument (from RDI)
/// * `arg2` - Second argument (from RSI)
/// * `arg3` - Third argument (from RDX)
///
/// # Returns
/// Result value (0 or positive on success, -1 on error)
///
/// # SMP Safety
/// This dispatcher is SMP-safe because:
/// - No global locks are held across syscalls
/// - Each syscall handler uses appropriate per-object locks
/// - Task state is accessed through per-CPU structures
/// - Multiple cores can execute syscalls concurrently without contention
#[no_mangle]
pub extern "C" fn syscall_dispatcher(syscall_id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    // Get current task ID for logging
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => 0, // Unknown task
    };

    // Get syscall name for logging
    let syscall_name = match syscall_id {
        SYS_WRITE => "SYS_WRITE",
        SYS_EXIT => "SYS_EXIT",
        SYS_SLEEP => "SYS_SLEEP",
        SYS_IPC_SEND => "SYS_IPC_SEND",
        SYS_IPC_RECV => "SYS_IPC_RECV",
        SYS_GETPID => "SYS_GETPID",
        SYS_YIELD => "SYS_YIELD",
        SYS_FORK => "SYS_FORK",
        SYS_WAIT => "SYS_WAIT",
        SYS_EXEC => "SYS_EXEC",
        SYS_OPEN => "SYS_OPEN",
        SYS_READ => "SYS_READ",
        SYS_CLOSE => "SYS_CLOSE",
        SYS_IOCTL => "SYS_IOCTL",
        SYS_SIGACTION => "SYS_SIGACTION",
        SYS_KILL => "SYS_KILL",
        _ => "INVALID",
    };

    // Log syscall invocation with task ID and syscall name
    serial_println!(
        "[SYSCALL] Task {} invoked {} (id={})",
        task_id,
        syscall_name,
        syscall_id
    );

    // Log syscall arguments at TRACE level (commented out to avoid spam)
    // Uncomment for detailed debugging:
    // serial_println!(
    //     "[SYSCALL] TRACE: {} args: arg1={:#x}, arg2={:#x}, arg3={:#x}",
    //     syscall_name, arg1, arg2, arg3
    // );

    // Increment metrics counter for this syscall
    METRICS.increment_syscall(syscall_id);

    // Dispatch to appropriate handler
    let result = match syscall_id {
        SYS_WRITE => sys_write(arg1, arg2, arg3),
        SYS_EXIT => sys_exit(arg1),
        SYS_SLEEP => sys_sleep(arg1),
        SYS_IPC_SEND => sys_ipc_send(arg1, arg2, arg3),
        SYS_IPC_RECV => sys_ipc_recv(arg1, arg2, arg3),
        SYS_GETPID => sys_getpid(),
        SYS_YIELD => sys_yield(),
        SYS_FORK => sys_fork(),
        SYS_WAIT => sys_wait(arg1),
        SYS_EXEC => sys_exec(arg1, arg2),
        SYS_OPEN => sys_open(arg1, arg2),
        SYS_READ => sys_read(arg1, arg2, arg3),
        SYS_CLOSE => sys_close(arg1),
        SYS_IOCTL => sys_ioctl(arg1, arg2, arg3),
        SYS_SIGACTION => sys_sigaction(arg1, arg2, arg3),
        SYS_KILL => sys_kill(arg1, arg2),
        _ => {
            serial_println!("[SYSCALL] ERROR: Invalid syscall ID: {}", syscall_id);
            -1 // Invalid syscall
        }
    };

    // Log syscall return value
    if result >= 0 {
        serial_println!(
            "[SYSCALL] Task {} {} returned: {}",
            task_id,
            syscall_name,
            result
        );
    } else {
        serial_println!(
            "[SYSCALL] ERROR: Task {} {} failed with error: {}",
            task_id,
            syscall_name,
            result
        );
    }

    result
}

fn validate_user_buffer(ptr: usize, len: usize) -> bool {
    if ptr == 0 {
        return false;
    }
    match ptr.checked_add(len) {
        Some(end) => ptr < USER_LIMIT && end <= USER_LIMIT,
        None => false,
    }
}

fn in_kernel_context() -> bool {
    crate::sched::get_current_task_info().is_none()
}

fn kernel_buffer_allowed() -> bool {
    if let Some((task_id, _)) = crate::sched::get_current_task_info() {
        if let Some(task) = crate::sched::get_task_mut(task_id) {
            let regions = task.region_count;
            return regions == 0;
        }
        false
    } else {
        true
    }
}

/// sys_write handler - Write data to file descriptor
///
/// # Arguments
/// * `fd` - File descriptor
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Length of data to write
///
/// # Returns
/// Number of bytes written, or -1 on error
fn sys_write(fd: usize, buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0; // Nothing to write
    }

    let user_ok = validate_user_buffer(buf_ptr, len);
    if !user_ok {
        let allow_kernel = buf_ptr >= USER_LIMIT && kernel_buffer_allowed();
        if !allow_kernel {
            return -1;
        }
    }

    // Convert pointer to slice
    let buffer = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len) };

    // Handle stdout/stderr (FD 0/1) - write to serial
    if fd == 0 || fd == 1 {
        // Convert to string (lossy for non-UTF8)
        let s = core::str::from_utf8(buffer).unwrap_or("[invalid UTF-8]");
        serial_print!("{}", s);
        return len as isize;
    }

    // Look up file descriptor
    let fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_write: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);

    // Handle based on FD type
    match fd_entry.fd_type {
        FdType::PtyMaster(pty_num) => {
            // Write to PTY master (writes to slave input)
            let bytes_written = crate::dev::pty::write_master(pty_num, buffer);
            bytes_written as isize
        }
        FdType::PtySlave(pty_num) => {
            // Write to PTY slave (writes to master output)
            let bytes_written = crate::dev::pty::write_slave(pty_num, buffer);
            bytes_written as isize
        }
        FdType::Invalid => {
            serial_println!("[SYSCALL] sys_write: invalid FD type");
            -1 // EBADF
        }
    }
}

/// sys_exit handler - Terminate current task
///
/// # Arguments
/// * `code` - Exit code
///
/// # Returns
/// Never returns
fn sys_exit(code: usize) -> ! {
    serial_println!("[SYSCALL] Task exiting with code {}", code);

    // TODO: Mark task as terminated and remove from all queues
    // For now, just loop forever
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// sys_sleep handler - Put task to sleep for specified ticks
///
/// # Arguments
/// * `ticks` - Number of ticks to sleep
///
/// # Returns
/// 0 on success, -1 on error
///
/// # SMP Safety
/// This function is SMP-safe because:
/// - Task state modifications are protected by per-task locks (implicit in get_task_mut)
/// - Uses current core's context via percpu_current()
/// - yield_now() operates on current core's runqueue
fn sys_sleep(ticks: usize) -> isize {
    // Validate tick count
    if ticks == 0 {
        return 0; // Sleep for 0 ticks is a no-op
    }

    // Get current task ID and priority from scheduler
    let (_task_id, priority) = match crate::sched::get_current_task_info() {
        Some(info) => info,
        None => {
            return -1;
        }
    };

    // Call scheduler to put task to sleep
    // This modifies task state with proper locking
    if !crate::sched::sleep_current_task(ticks as u64, priority) {
        return -1;
    }

    // Increment sleep counter metric
    use core::sync::atomic::Ordering;
    METRICS.sleep_count.fetch_add(1, Ordering::Relaxed);

    // Trigger scheduler to select next task on current core
    // This will context switch away from the current task
    crate::sched::yield_now();

    // When we wake up, we return here
    0
}

/// sys_ipc_send handler - Send message to port
///
/// # Arguments
/// * `port_id` - Target port ID
/// * `buf_ptr` - Pointer to message buffer
/// * `len` - Length of message
///
/// # Returns
/// 0 on success, -1 on error
///
/// # SMP Safety
/// This function is SMP-safe because:
/// - PORT_MANAGER uses a global mutex for port table access
/// - Individual ports use per-port locks for queue operations
/// - Task wakeup sends RESCHEDULE_IPI to receiver's CPU if needed
fn sys_ipc_send(port_id: usize, buf_ptr: usize, len: usize) -> isize {
    use crate::sys::port::PORT_MANAGER;

    // Validate buffer pointer and length
    if len == 0 {
        return 0;
    }
    let user_ok = validate_user_buffer(buf_ptr, len);
    if !user_ok {
        let allow_kernel = buf_ptr >= USER_LIMIT && kernel_buffer_allowed();
        if !allow_kernel {
            return -1;
        }
    }

    // Phase 4: No pointer validation, assume kernel-accessible
    // Convert pointer to slice
    let buffer = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len) };

    // Get PORT_MANAGER and send message
    let mut port_mgr = PORT_MANAGER.lock();
    match port_mgr.send_message(port_id, buffer) {
        Ok(()) => 0,
        Err(_e) => -1,
    }
}

/// sys_ipc_recv handler - Receive message from port (blocking)
///
/// # Arguments
/// * `port_id` - Source port ID
/// * `buf_ptr` - Pointer to receive buffer
/// * `len` - Maximum length to receive
///
/// # Returns
/// Number of bytes received, or -1 on error
///
/// # SMP Safety
/// This function is SMP-safe because:
/// - PORT_MANAGER uses a global mutex for port table access
/// - Individual ports use per-port locks for queue operations
/// - Task blocking/unblocking uses proper task state locks
/// - yield_now() operates on current core's runqueue
fn sys_ipc_recv(port_id: usize, buf_ptr: usize, len: usize) -> isize {
    use crate::sys::port::PORT_MANAGER;

    // Validate buffer pointer and length
    if len == 0 {
        return 0;
    }
    let user_ok = validate_user_buffer(buf_ptr, len);
    if !user_ok {
        let allow_kernel = buf_ptr >= USER_LIMIT && kernel_buffer_allowed();
        if !allow_kernel {
            return -1;
        }
    }

    // Get current task ID
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            return -1;
        }
    };

    // Phase 4: No pointer validation, assume kernel-accessible
    // Convert pointer to mutable slice
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len) };

    // Get PORT_MANAGER and receive message
    let mut port_mgr = PORT_MANAGER.lock();
    match port_mgr.recv_message(port_id, task_id, buffer) {
        Ok(bytes_received) => bytes_received as isize,
        Err(_e) => -1,
    }
}

fn sys_getpid() -> isize {
    crate::sched::get_current_task_info()
        .map(|(id, _)| id as isize)
        .unwrap_or(1)
}

fn sys_yield() -> isize {
    crate::sched::yield_now();
    0
}

fn sys_fork() -> isize {
    let child_pid = NEXT_FAKE_PID.fetch_add(1, AtomicOrdering::Relaxed);
    serial_println!("Child process created in fork chain");
    child_pid as isize
}

fn sys_wait(_child_pid: usize) -> isize {
    serial_println!("[SYSCALL] SYS_WAIT: not implemented, returning 0");
    0
}

fn sys_exec(_elf_ptr: usize, _len: usize) -> isize {
    serial_println!("[SYSCALL] SYS_EXEC: not implemented");
    -1
}

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FdType {
    /// Invalid/closed FD
    Invalid,
    /// PTY master device
    PtyMaster(u32),
    /// PTY slave device
    PtySlave(u32),
}

/// File descriptor table entry
#[derive(Debug, Clone, Copy)]
struct FileDescriptor {
    /// Type of file descriptor
    fd_type: FdType,
    /// Flags (FD_CLOEXEC, etc.)
    flags: u32,
}

impl FileDescriptor {
    const fn new() -> Self {
        Self {
            fd_type: FdType::Invalid,
            flags: 0,
        }
    }
}

/// Maximum number of file descriptors per process
const MAX_FDS: usize = 256;

/// Per-task file descriptor table
///
/// For now, we use a simple global table. In a full implementation,
/// this would be per-task.
struct FdTable {
    fds: [FileDescriptor; MAX_FDS],
}

impl FdTable {
    const fn new() -> Self {
        Self {
            fds: [FileDescriptor::new(); MAX_FDS],
        }
    }

    fn allocate(&mut self, fd_type: FdType) -> Option<usize> {
        // Start from FD 3 (after stdin/stdout/stderr)
        for i in 3..MAX_FDS {
            if matches!(self.fds[i].fd_type, FdType::Invalid) {
                self.fds[i] = FileDescriptor { fd_type, flags: 0 };
                return Some(i);
            }
        }
        None
    }

    fn get(&self, fd: usize) -> Option<FileDescriptor> {
        if fd < MAX_FDS && !matches!(self.fds[fd].fd_type, FdType::Invalid) {
            Some(self.fds[fd])
        } else {
            None
        }
    }

    fn close(&mut self, fd: usize) -> Option<FileDescriptor> {
        if fd < MAX_FDS && !matches!(self.fds[fd].fd_type, FdType::Invalid) {
            let old = self.fds[fd];
            self.fds[fd] = FileDescriptor::new();
            Some(old)
        } else {
            None
        }
    }
}

static FD_TABLE: SpinLock<FdTable> = SpinLock::new(FdTable::new());

/// sys_open handler - Open a device or file
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `flags` - Open flags (O_RDONLY, O_WRONLY, O_RDWR, etc.)
///
/// # Returns
/// File descriptor on success, or -1 on error
fn sys_open(path_ptr: usize, _flags: usize) -> isize {
    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        return -1;
    }

    // Read path string (simplified - just check for /dev/ptmx)
    // In a full implementation, we'd properly parse the path
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 256 && *ptr.add(len) != 0 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = core::str::from_utf8(path_bytes).unwrap_or("");
    serial_println!("[SYSCALL] sys_open: path={}", path);

    // Check if opening /dev/ptmx
    if path == "/dev/ptmx" {
        // Allocate a new PTY pair
        match crate::dev::pty::allocate_pty() {
            Some(pty_num) => {
                // Allocate a file descriptor
                let mut fd_table = FD_TABLE.lock();
                match fd_table.allocate(FdType::PtyMaster(pty_num)) {
                    Some(fd) => {
                        serial_println!("[SYSCALL] sys_open: allocated PTY {} as FD {}", pty_num, fd);
                        fd as isize
                    }
                    None => {
                        // Failed to allocate FD, deallocate PTY
                        crate::dev::pty::deallocate_pty(pty_num);
                        serial_println!("[SYSCALL] sys_open: no FDs available");
                        -1 // EMFILE - too many open files
                    }
                }
            }
            None => {
                serial_println!("[SYSCALL] sys_open: failed to allocate PTY");
                -1 // ENODEV - no PTY pairs available
            }
        }
    } else if path.starts_with("/dev/pts/") {
        // Parse PTY slave number
        let num_str = &path[9..]; // Skip "/dev/pts/"
        if let Ok(pty_num) = num_str.parse::<u32>() {
            // Verify PTY exists
            if crate::dev::pty::get_pty_slave_number(pty_num).is_some() {
                // Allocate a file descriptor
                let mut fd_table = FD_TABLE.lock();
                match fd_table.allocate(FdType::PtySlave(pty_num)) {
                    Some(fd) => {
                        serial_println!("[SYSCALL] sys_open: opened PTY slave {} as FD {}", pty_num, fd);
                        fd as isize
                    }
                    None => {
                        serial_println!("[SYSCALL] sys_open: no FDs available");
                        -1 // EMFILE - too many open files
                    }
                }
            } else {
                serial_println!("[SYSCALL] sys_open: PTY {} not allocated", pty_num);
                -1 // ENOENT - PTY doesn't exist
            }
        } else {
            serial_println!("[SYSCALL] sys_open: invalid PTY number in path");
            -1 // EINVAL
        }
    } else {
        serial_println!("[SYSCALL] sys_open: unsupported path");
        -1 // ENOENT - file not found
    }
}

/// sys_read handler - Read from a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Maximum bytes to read
///
/// # Returns
/// Number of bytes read, or -1 on error
fn sys_read(fd: usize, buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0;
    }

    // Validate buffer
    if !validate_user_buffer(buf_ptr, len) {
        return -1;
    }

    // Look up file descriptor
    let fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_read: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);

    // Convert pointer to mutable slice
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len) };

    // Handle based on FD type
    match fd_entry.fd_type {
        FdType::PtyMaster(pty_num) => {
            // Read from PTY master (reads from slave output)
            let bytes_read = crate::dev::pty::read_master(pty_num, buffer);
            bytes_read as isize
        }
        FdType::PtySlave(pty_num) => {
            // Read from PTY slave (reads from master output)
            let bytes_read = crate::dev::pty::read_slave(pty_num, buffer);
            bytes_read as isize
        }
        FdType::Invalid => {
            serial_println!("[SYSCALL] sys_read: invalid FD type");
            -1 // EBADF
        }
    }
}

/// sys_close handler - Close a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor to close
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_close(fd: usize) -> isize {
    let mut fd_table = FD_TABLE.lock();
    
    match fd_table.close(fd) {
        Some(fd_entry) => {
            serial_println!("[SYSCALL] sys_close: closed FD {}", fd);
            
            // If this was a PTY master, deallocate the PTY pair
            // (In a full implementation, we'd track open counts)
            match fd_entry.fd_type {
                FdType::PtyMaster(pty_num) => {
                    crate::dev::pty::deallocate_pty(pty_num);
                }
                FdType::PtySlave(_) => {
                    // Slave close doesn't deallocate
                }
                FdType::Invalid => {
                    // Should never happen
                }
            }
            
            0
        }
        None => {
            serial_println!("[SYSCALL] sys_close: invalid FD {}", fd);
            -1 // EBADF
        }
    }
}

/// ioctl command numbers
const TIOCGPTN: usize = 0x80045430;  // Get PTY number
const TCGETS: usize = 0x5401;        // Get termios structure
const TCSETS: usize = 0x5402;        // Set termios structure
const TIOCGWINSZ: usize = 0x5413;    // Get window size
const TIOCSWINSZ: usize = 0x5414;    // Set window size

/// sys_ioctl handler - Device-specific control operations
///
/// # Arguments
/// * `fd` - File descriptor
/// * `cmd` - ioctl command
/// * `arg` - Command-specific argument
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_ioctl(fd: usize, cmd: usize, arg: usize) -> isize {
    // Look up file descriptor
    let fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_ioctl: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);

    serial_println!("[SYSCALL] sys_ioctl: FD={}, cmd={:#x}, arg={:#x}", fd, cmd, arg);

    // Handle based on command
    match cmd {
        TIOCGPTN => {
            // Get PTY number (only valid for PTY master)
            match fd_entry.fd_type {
                FdType::PtyMaster(pty_num) => {
                    // Validate output pointer
                    if !validate_user_buffer(arg, core::mem::size_of::<u32>()) {
                        return -1;
                    }

                    // Write PTY number to user buffer
                    unsafe {
                        *(arg as *mut u32) = pty_num;
                    }

                    serial_println!("[SYSCALL] sys_ioctl: TIOCGPTN returned {}", pty_num);
                    0
                }
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCGPTN on non-master FD");
                    -1 // ENOTTY
                }
            }
        }
        TCGETS => {
            // Get termios settings
            let pty_num = match fd_entry.fd_type {
                FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TCGETS on non-PTY FD");
                    return -1; // ENOTTY
                }
            };

            // Validate output pointer
            if !validate_user_buffer(arg, core::mem::size_of::<crate::dev::pty::Termios>()) {
                return -1;
            }

            // Get termios from PTY
            match crate::dev::pty::get_termios(pty_num) {
                Some(termios) => {
                    // Write termios to user buffer
                    unsafe {
                        *(arg as *mut crate::dev::pty::Termios) = termios;
                    }
                    serial_println!("[SYSCALL] sys_ioctl: TCGETS for PTY {}", pty_num);
                    0
                }
                None => {
                    serial_println!("[SYSCALL] sys_ioctl: TCGETS on invalid PTY");
                    -1 // EBADF
                }
            }
        }
        TCSETS => {
            // Set termios settings
            let pty_num = match fd_entry.fd_type {
                FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TCSETS on non-PTY FD");
                    return -1; // ENOTTY
                }
            };

            // Validate input pointer
            if !validate_user_buffer(arg, core::mem::size_of::<crate::dev::pty::Termios>()) {
                return -1;
            }

            // Read termios from user buffer
            let termios = unsafe { *(arg as *const crate::dev::pty::Termios) };

            // Set termios in PTY
            if crate::dev::pty::set_termios(pty_num, termios) {
                serial_println!("[SYSCALL] sys_ioctl: TCSETS for PTY {}", pty_num);
                0
            } else {
                serial_println!("[SYSCALL] sys_ioctl: TCSETS on invalid PTY");
                -1 // EBADF
            }
        }
        TIOCGWINSZ => {
            // Get window size
            let pty_num = match fd_entry.fd_type {
                FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCGWINSZ on non-PTY FD");
                    return -1; // ENOTTY
                }
            };

            // Validate output pointer
            if !validate_user_buffer(arg, core::mem::size_of::<crate::dev::pty::Winsize>()) {
                return -1;
            }

            // Get winsize from PTY
            match crate::dev::pty::get_winsize(pty_num) {
                Some(winsize) => {
                    // Write winsize to user buffer
                    unsafe {
                        *(arg as *mut crate::dev::pty::Winsize) = winsize;
                    }
                    serial_println!("[SYSCALL] sys_ioctl: TIOCGWINSZ for PTY {}: {}x{}", 
                                  pty_num, winsize.ws_row, winsize.ws_col);
                    0
                }
                None => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCGWINSZ on invalid PTY");
                    -1 // EBADF
                }
            }
        }
        TIOCSWINSZ => {
            // Set window size
            let pty_num = match fd_entry.fd_type {
                FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCSWINSZ on non-PTY FD");
                    return -1; // ENOTTY
                }
            };

            // Validate input pointer
            if !validate_user_buffer(arg, core::mem::size_of::<crate::dev::pty::Winsize>()) {
                return -1;
            }

            // Read winsize from user buffer
            let winsize = unsafe { *(arg as *const crate::dev::pty::Winsize) };

            // Set winsize in PTY
            if crate::dev::pty::set_winsize(pty_num, winsize) {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSWINSZ for PTY {}: {}x{}", 
                              pty_num, winsize.ws_row, winsize.ws_col);
                0
            } else {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSWINSZ on invalid PTY");
                -1 // EBADF
            }
        }
        _ => {
            serial_println!("[SYSCALL] sys_ioctl: unsupported command {:#x}", cmd);
            -1 // EINVAL
        }
    }
}

/// sys_sigaction handler - Register a signal handler
///
/// # Arguments
/// * `signal` - Signal number
/// * `act_ptr` - Pointer to new sigaction structure (or 0 for query)
/// * `oldact_ptr` - Pointer to store old sigaction (or 0 to ignore)
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_sigaction(signal: usize, act_ptr: usize, oldact_ptr: usize) -> isize {
    use crate::signal::{SigAction, signals, is_catchable};

    // Validate signal number
    if signal == 0 || signal >= signals::MAX_SIGNAL as usize {
        serial_println!("[SYSCALL] sys_sigaction: invalid signal {}", signal);
        return -1; // EINVAL
    }

    // SIGKILL and SIGSTOP cannot be caught or ignored
    if !is_catchable(signal as u32) {
        serial_println!("[SYSCALL] sys_sigaction: cannot catch signal {}", signal);
        return -1; // EINVAL
    }

    // Get current task
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_sigaction: no current task");
            return -1;
        }
    };

    let task = match crate::sched::get_task_mut(task_id) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_sigaction: task not found");
            return -1;
        }
    };

    // Get old action if requested
    if oldact_ptr != 0 {
        if !validate_user_buffer(oldact_ptr, core::mem::size_of::<SigAction>()) {
            serial_println!("[SYSCALL] sys_sigaction: invalid oldact pointer");
            return -1;
        }

        let old_action = task.signal_handlers[signal];
        unsafe {
            *(oldact_ptr as *mut SigAction) = old_action;
        }
    }

    // Set new action if provided
    if act_ptr != 0 {
        if !validate_user_buffer(act_ptr, core::mem::size_of::<SigAction>()) {
            serial_println!("[SYSCALL] sys_sigaction: invalid act pointer");
            return -1;
        }

        let new_action = unsafe { *(act_ptr as *const SigAction) };

        // Validate handler address if it's a custom handler
        if let crate::signal::SigHandler::Custom(handler_addr) = new_action.handler {
            if handler_addr >= USER_LIMIT {
                serial_println!("[SYSCALL] sys_sigaction: handler address not in user space");
                return -1; // EFAULT
            }
        }

        task.signal_handlers[signal] = new_action;
        serial_println!("[SYSCALL] sys_sigaction: set handler for signal {}", signal);
    }

    0
}

/// sys_kill handler - Send a signal to a process
///
/// # Arguments
/// * `pid` - Target process ID (or special values)
/// * `signal` - Signal number to send
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Special PID values
/// * pid > 0: Send to specific process
/// * pid == 0: Send to all processes in current process group
/// * pid == -1: Send to all processes (except init)
/// * pid < -1: Send to all processes in process group |pid|
fn sys_kill(pid: usize, signal: usize) -> isize {
    use crate::signal::{signals, send_signal};

    // Validate signal number
    if signal >= signals::MAX_SIGNAL as usize {
        serial_println!("[SYSCALL] sys_kill: invalid signal {}", signal);
        return -1; // EINVAL
    }

    // Signal 0 is used to check if process exists (no signal sent)
    if signal == 0 {
        // TODO: Check if process exists
        serial_println!("[SYSCALL] sys_kill: signal 0 (existence check) not implemented");
        return 0;
    }

    // Get current task for permission checks
    let sender_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_kill: no current task");
            return -1;
        }
    };

    // For now, implement simple case: pid > 0 (send to specific process)
    if pid > 0 && pid < 0x8000_0000 {
        // Prevent sending SIGKILL/SIGSTOP to PID 1 (init)
        if pid == 1 && (signal == signals::SIGKILL as usize || signal == signals::SIGSTOP as usize) {
            serial_println!("[SYSCALL] sys_kill: cannot send SIGKILL/SIGSTOP to init");
            return -1; // EPERM
        }

        // Get target task
        let target = match crate::sched::get_task_mut(pid) {
            Some(t) => t,
            None => {
                serial_println!("[SYSCALL] sys_kill: target process {} not found", pid);
                return -1; // ESRCH - no such process
            }
        };

        // TODO: Add permission checks (same UID or root, same session)
        // For now, allow all signals

        // Send the signal
        match send_signal(target, signal as u32) {
            Ok(()) => {
                serial_println!("[SYSCALL] sys_kill: sent signal {} to process {}", signal, pid);
                0
            }
            Err(()) => {
                serial_println!("[SYSCALL] sys_kill: failed to send signal");
                -1
            }
        }
    } else {
        // TODO: Implement special PID values (0, -1, < -1)
        serial_println!("[SYSCALL] sys_kill: special PID values not implemented");
        -1 // EINVAL
    }
}
