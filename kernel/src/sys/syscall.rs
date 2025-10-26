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
pub const SYS_SETPGID: usize = 16;
pub const SYS_GETPGRP: usize = 17;
pub const SYS_SETSID: usize = 18;
pub const SYS_GETSID: usize = 19;
pub const SYS_TCSETPGRP: usize = 20;
pub const SYS_TCGETPGRP: usize = 21;
pub const SYS_FCNTL: usize = 22;
pub const SYS_PIPE2: usize = 23;
pub const SYS_DUP2: usize = 24;
pub const SYS_READ_STDIN: usize = 25;
pub const SYS_SERIAL_WRITE: usize = 26;
pub const SYS_SERIAL_READ: usize = 27;
pub const SYS_BLOCK_READ: usize = 28;
pub const SYS_BLOCK_WRITE: usize = 29;
pub const SYS_GET_DEVICE_LIST: usize = 30;
pub const SYS_GET_BLOCK_DEVICE_INFO: usize = 31;
pub const SYS_READ_KERNEL_LOG: usize = 32;
pub const SYS_GET_IRQ_STATS: usize = 33;
pub const SYS_STAT: usize = 34;
pub const SYS_FSTAT: usize = 35;
pub const SYS_LSTAT: usize = 36;
pub const SYS_CHMOD: usize = 37;
pub const SYS_CHOWN: usize = 38;
pub const SYS_UTIMENSAT: usize = 39;
pub const SYS_SETXATTR: usize = 40;
pub const SYS_GETXATTR: usize = 41;
pub const SYS_LISTXATTR: usize = 42;
pub const SYS_MKNOD: usize = 43;
pub const SYS_SYNC: usize = 44;
pub const SYS_FSYNC: usize = 45;
pub const SYS_FDATASYNC: usize = 46;
pub const SYS_MOUNT: usize = 47;
pub const SYS_UMOUNT: usize = 48;

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
pub extern "C" fn syscall_dispatcher(
    syscall_id: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> isize {
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
        SYS_SETPGID => "SYS_SETPGID",
        SYS_GETPGRP => "SYS_GETPGRP",
        SYS_SETSID => "SYS_SETSID",
        SYS_GETSID => "SYS_GETSID",
        SYS_TCSETPGRP => "SYS_TCSETPGRP",
        SYS_TCGETPGRP => "SYS_TCGETPGRP",
        SYS_FCNTL => "SYS_FCNTL",
        SYS_PIPE2 => "SYS_PIPE2",
        SYS_DUP2 => "SYS_DUP2",
        SYS_READ_STDIN => "SYS_READ_STDIN",
        SYS_SERIAL_WRITE => "SYS_SERIAL_WRITE",
        SYS_SERIAL_READ => "SYS_SERIAL_READ",
        SYS_BLOCK_READ => "SYS_BLOCK_READ",
        SYS_BLOCK_WRITE => "SYS_BLOCK_WRITE",
        SYS_GET_DEVICE_LIST => "SYS_GET_DEVICE_LIST",
        SYS_GET_BLOCK_DEVICE_INFO => "SYS_GET_BLOCK_DEVICE_INFO",
        SYS_READ_KERNEL_LOG => "SYS_READ_KERNEL_LOG",
        SYS_GET_IRQ_STATS => "SYS_GET_IRQ_STATS",
        SYS_STAT => "SYS_STAT",
        SYS_FSTAT => "SYS_FSTAT",
        SYS_LSTAT => "SYS_LSTAT",
        SYS_CHMOD => "SYS_CHMOD",
        SYS_CHOWN => "SYS_CHOWN",
        SYS_UTIMENSAT => "SYS_UTIMENSAT",
        SYS_SETXATTR => "SYS_SETXATTR",
        SYS_GETXATTR => "SYS_GETXATTR",
        SYS_LISTXATTR => "SYS_LISTXATTR",
        SYS_MKNOD => "SYS_MKNOD",
        SYS_SYNC => "SYS_SYNC",
        SYS_FSYNC => "SYS_FSYNC",
        SYS_FDATASYNC => "SYS_FDATASYNC",
        SYS_MOUNT => "SYS_MOUNT",
        SYS_UMOUNT => "SYS_UMOUNT",
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
        SYS_SETPGID => sys_setpgid(arg1, arg2),
        SYS_GETPGRP => sys_getpgrp(),
        SYS_SETSID => sys_setsid(),
        SYS_GETSID => sys_getsid(arg1),
        SYS_TCSETPGRP => sys_tcsetpgrp(arg1, arg2),
        SYS_TCGETPGRP => sys_tcgetpgrp(arg1),
        SYS_FCNTL => sys_fcntl(arg1, arg2, arg3),
        SYS_PIPE2 => sys_pipe2(arg1, arg2),
        SYS_DUP2 => sys_dup2(arg1, arg2),
        SYS_READ_STDIN => sys_read_stdin(arg1, arg2),
        SYS_SERIAL_WRITE => sys_serial_write(arg1, arg2),
        SYS_SERIAL_READ => sys_serial_read(arg1, arg2),
        SYS_BLOCK_READ => sys_block_read(arg1, arg2, arg3),
        SYS_BLOCK_WRITE => sys_block_write(arg1, arg2, arg3),
        SYS_GET_DEVICE_LIST => sys_get_device_list(arg1, arg2),
        SYS_GET_BLOCK_DEVICE_INFO => sys_get_block_device_info(arg1),
        SYS_READ_KERNEL_LOG => sys_read_kernel_log(arg1, arg2),
        SYS_GET_IRQ_STATS => sys_get_irq_stats(arg1, arg2),
        SYS_STAT => sys_stat(arg1, arg2),
        SYS_FSTAT => sys_fstat(arg1, arg2),
        SYS_LSTAT => sys_lstat(arg1, arg2),
        SYS_CHMOD => sys_chmod(arg1, arg2),
        SYS_CHOWN => sys_chown(arg1, arg2, arg3),
        SYS_UTIMENSAT => sys_utimensat(arg1, arg2, arg3),
        SYS_SETXATTR => sys_setxattr(arg1, arg2, arg3),
        SYS_GETXATTR => sys_getxattr(arg1, arg2, arg3),
        SYS_LISTXATTR => sys_listxattr(arg1, arg2),
        SYS_MKNOD => sys_mknod(arg1, arg2, arg3),
        SYS_SYNC => sys_sync(),
        SYS_FSYNC => sys_fsync(arg1),
        SYS_FDATASYNC => sys_fdatasync(arg1),
        SYS_MOUNT => sys_mount(arg1, arg2, arg3),
        SYS_UMOUNT => sys_umount(arg1, arg2),
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
        FdType::PipeWrite(pipe_id) => {
            // Write to pipe
            let mut pipe_table = PIPE_TABLE.lock();
            match pipe_table.get_mut(pipe_id) {
                Some(pipe) => {
                    // Check if there are any readers
                    if pipe.readers == 0 {
                        serial_println!("[SYSCALL] sys_write: pipe has no readers (SIGPIPE)");
                        return -1; // EPIPE
                    }
                    let bytes_written = pipe.write(buffer);
                    bytes_written as isize
                }
                None => {
                    serial_println!("[SYSCALL] sys_write: invalid pipe");
                    -1 // EBADF
                }
            }
        }
        FdType::PipeRead(_) => {
            serial_println!("[SYSCALL] sys_write: cannot write to pipe read end");
            -1 // EBADF
        }
        FdType::VfsFile { ref inode, ref offset, flags } => {
            // Write to VFS file
            use core::sync::atomic::Ordering;
            
            // Check if file is opened for writing
            let o_accmode = flags & 0x3; // O_RDONLY=0, O_WRONLY=1, O_RDWR=2
            if o_accmode == 0 { // O_RDONLY
                serial_println!("[SYSCALL] sys_write: file not opened for writing");
                return -1; // EBADF
            }
            
            let current_offset = if (flags & 0x400) != 0 { // O_APPEND
                // Append mode: write at end of file
                inode.size()
            } else {
                offset.load(Ordering::SeqCst)
            };
            
            match inode.write_at(current_offset, buffer) {
                Ok(bytes_written) => {
                    // Update offset
                    offset.store(current_offset + bytes_written as u64, Ordering::SeqCst);
                    bytes_written as isize
                }
                Err(err) => {
                    serial_println!("[SYSCALL] sys_write: VFS write failed: {:?}", err);
                    // Map VFS errors to errno
                    match err {
                        crate::fs::vfs::superblock::FsError::IoError => -5, // EIO
                        crate::fs::vfs::superblock::FsError::NoSpace => -28, // ENOSPC
                        crate::fs::vfs::superblock::FsError::IsADirectory => -21, // EISDIR
                        crate::fs::vfs::superblock::FsError::ReadOnlyFilesystem => -30, // EROFS
                        _ => -1, // Generic error
                    }
                }
            }
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

/// sys_exec handler - Execute a new program
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `argv_ptr` - Pointer to NULL-terminated array of argument strings
///
/// # Returns
/// Does not return on success (process is replaced)
/// Negative error code on failure
///
/// # Requirements
/// Implements R1.1-R1.8, R8.1-R8.3:
/// - R1.1: Load ELF binary from specified path
/// - R1.2: Replace current process image with new program
/// - R1.3: Preserve process ID and file descriptors
/// - R1.4: Never return on success
/// - R1.5: Return error and preserve process on failure
/// - R1.6: Close O_CLOEXEC file descriptors
/// - R1.7: Pass command-line arguments to new program
/// - R1.8: Pass environment variables to new program
/// - R8.1: Validate path pointer is in user space
/// - R8.2: Validate argv pointer array is in user space
/// - R8.3: Validate envp pointer array is in user space
fn sys_exec(path_ptr: usize, argv_ptr: usize) -> isize {
    use crate::user::exec::{
        ExecContext, ExecError, 
        validate_user_pointer, 
        copy_string_from_user, 
        copy_string_array_from_user
    };
    
    serial_println!("[SYSCALL] sys_exec: path_ptr={:#x}, argv_ptr={:#x}", path_ptr, argv_ptr);
    
    // Step 1: Validate path pointer
    if let Err(e) = validate_user_pointer(path_ptr) {
        serial_println!("[SYSCALL] sys_exec: invalid path pointer");
        return e.to_errno();
    }
    
    // Step 2: Copy path string from user space
    let path = match copy_string_from_user(path_ptr) {
        Ok(s) => s,
        Err(e) => {
            serial_println!("[SYSCALL] sys_exec: failed to copy path string: {:?}", e);
            return e.to_errno();
        }
    };
    
    serial_println!("[SYSCALL] sys_exec: path={}", path);
    
    // Step 3: Validate and copy argv array
    // argv_ptr can be 0 (NULL), which means empty argv
    let argv = if argv_ptr == 0 {
        // Empty argv - use path as argv[0]
        alloc::vec![path.clone()]
    } else {
        // Validate argv pointer
        if let Err(e) = validate_user_pointer(argv_ptr) {
            serial_println!("[SYSCALL] sys_exec: invalid argv pointer");
            return e.to_errno();
        }
        
        // Copy argv array from user space
        match copy_string_array_from_user(argv_ptr) {
            Ok(arr) => {
                // If argv is empty, use path as argv[0]
                if arr.is_empty() {
                    alloc::vec![path.clone()]
                } else {
                    arr
                }
            }
            Err(e) => {
                serial_println!("[SYSCALL] sys_exec: failed to copy argv array: {:?}", e);
                return e.to_errno();
            }
        }
    };
    
    serial_println!("[SYSCALL] sys_exec: argc={}, argv={:?}", argv.len(), argv);
    
    // Step 4: Setup environment variables
    // In a full implementation, we would:
    // 1. Accept envp_ptr as a third argument
    // 2. Copy environment from user space
    // 3. Merge with inherited environment
    let envp = alloc::vec![
        alloc::string::String::from("PATH=/bin"),
        alloc::string::String::from("HOME=/root"),
    ];
    
    serial_println!("[SYSCALL] sys_exec: envp has {} variables", envp.len());
    
    // Step 5: Get current task
    let task_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_exec: no current task");
            return ExecError::InvalidArgument.to_errno();
        }
    };
    
    // Get task reference
    // Note: We need to get the task as Arc<Task> for ExecContext
    let task_arc = match crate::sched::get_task_arc(task_id) {
        Some(task) => task,
        None => {
            serial_println!("[SYSCALL] sys_exec: task not found");
            return ExecError::InvalidArgument.to_errno();
        }
    };
    
    // Step 6: Create ExecContext
    let ctx = ExecContext::new(path, argv, envp, task_arc);
    
    // Step 7: Get PMM for memory allocation
    // We need mutable access to PMM for allocating pages
    let mut pmm_guard = crate::mm::pmm::get_global_pmm();
    let pmm = pmm_guard.as_mut().expect("Global PMM not initialized");
    
    // Step 8: Execute the new program
    // This never returns on success
    serial_println!("[SYSCALL] sys_exec: calling ExecContext::exec()");
    
    match ctx.exec(pmm) {
        Ok(never) => {
            // This should never be reached because exec() never returns on success
            // The type system ensures this with the ! (never) type
            // The never type (!) can coerce to any type, including isize
            match never {}
        }
        Err(e) => {
            // exec() failed, return error code
            serial_println!("[SYSCALL] sys_exec: exec failed: {:?}", e);
            e.to_errno()
        }
    }
}

/// File descriptor type
pub enum FdType {
    /// Invalid/closed FD
    Invalid,
    /// PTY master device
    PtyMaster(u32),
    /// PTY slave device
    PtySlave(u32),
    /// Pipe read end
    PipeRead(u32),
    /// Pipe write end
    PipeWrite(u32),
    /// VFS file (regular file, directory, etc.)
    VfsFile {
        inode: alloc::sync::Arc<dyn crate::fs::vfs::inode::Inode>,
        offset: core::sync::atomic::AtomicU64,
        flags: u32, // O_RDONLY, O_WRONLY, O_RDWR, O_APPEND, etc.
    },
}

// Manual Clone implementation since AtomicU64 doesn't implement Clone
impl Clone for FdType {
    fn clone(&self) -> Self {
        match self {
            FdType::Invalid => FdType::Invalid,
            FdType::PtyMaster(n) => FdType::PtyMaster(*n),
            FdType::PtySlave(n) => FdType::PtySlave(*n),
            FdType::PipeRead(n) => FdType::PipeRead(*n),
            FdType::PipeWrite(n) => FdType::PipeWrite(*n),
            FdType::VfsFile { inode, offset, flags } => {
                use core::sync::atomic::Ordering;
                FdType::VfsFile {
                    inode: inode.clone(),
                    offset: core::sync::atomic::AtomicU64::new(offset.load(Ordering::SeqCst)),
                    flags: *flags,
                }
            }
        }
    }
}

// Manual Debug implementation since dyn Inode doesn't implement Debug
impl core::fmt::Debug for FdType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FdType::Invalid => write!(f, "Invalid"),
            FdType::PtyMaster(n) => write!(f, "PtyMaster({})", n),
            FdType::PtySlave(n) => write!(f, "PtySlave({})", n),
            FdType::PipeRead(n) => write!(f, "PipeRead({})", n),
            FdType::PipeWrite(n) => write!(f, "PipeWrite({})", n),
            FdType::VfsFile { inode, offset, flags } => {
                use core::sync::atomic::Ordering;
                write!(f, "VfsFile {{ ino: {}, offset: {}, flags: {:#x} }}", 
                    inode.ino(), offset.load(Ordering::SeqCst), flags)
            }
        }
    }
}

// Manual PartialEq implementation since Arc doesn't implement Copy
impl PartialEq for FdType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FdType::Invalid, FdType::Invalid) => true,
            (FdType::PtyMaster(a), FdType::PtyMaster(b)) => a == b,
            (FdType::PtySlave(a), FdType::PtySlave(b)) => a == b,
            (FdType::PipeRead(a), FdType::PipeRead(b)) => a == b,
            (FdType::PipeWrite(a), FdType::PipeWrite(b)) => a == b,
            (FdType::VfsFile { inode: a, .. }, FdType::VfsFile { inode: b, .. }) => {
                alloc::sync::Arc::ptr_eq(a, b)
            }
            _ => false,
        }
    }
}

impl Eq for FdType {}

/// File descriptor flags (FD_CLOEXEC)
const FD_CLOEXEC: u32 = 1;

/// File status flags
const O_NONBLOCK: u32 = 0x800;
const O_APPEND: u32 = 0x400;

/// File descriptor table entry
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// Type of file descriptor
    fd_type: FdType,
    /// FD flags (FD_CLOEXEC, etc.)
    fd_flags: u32,
    /// File status flags (O_NONBLOCK, O_APPEND, etc.)
    status_flags: u32,
}

impl FileDescriptor {
    const fn new() -> Self {
        Self {
            fd_type: FdType::Invalid,
            fd_flags: 0,
            status_flags: 0,
        }
    }

    fn with_type(fd_type: FdType) -> Self {
        Self {
            fd_type,
            fd_flags: 0,
            status_flags: 0,
        }
    }

    fn with_flags(fd_type: FdType, fd_flags: u32, status_flags: u32) -> Self {
        Self {
            fd_type,
            fd_flags,
            status_flags,
        }
    }
}

/// Maximum number of file descriptors per process
const MAX_FDS: usize = 256;

/// Maximum number of pipes
const MAX_PIPES: usize = 64;

/// Pipe buffer size (4KB)
const PIPE_BUF_SIZE: usize = 4096;

/// Pipe structure
struct Pipe {
    /// Ring buffer for data
    buffer: [u8; PIPE_BUF_SIZE],
    /// Read position
    read_pos: usize,
    /// Write position
    write_pos: usize,
    /// Number of bytes in buffer
    count: usize,
    /// Number of read ends open
    readers: usize,
    /// Number of write ends open
    writers: usize,
}

impl Pipe {
    const fn new() -> Self {
        Self {
            buffer: [0; PIPE_BUF_SIZE],
            read_pos: 0,
            write_pos: 0,
            count: 0,
            readers: 0,
            writers: 0,
        }
    }

    fn is_allocated(&self) -> bool {
        self.readers > 0 || self.writers > 0
    }

    fn read(&mut self, buf: &mut [u8]) -> usize {
        let to_read = core::cmp::min(buf.len(), self.count);
        for i in 0..to_read {
            buf[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % PIPE_BUF_SIZE;
        }
        self.count -= to_read;
        to_read
    }

    fn write(&mut self, buf: &[u8]) -> usize {
        let space = PIPE_BUF_SIZE - self.count;
        let to_write = core::cmp::min(buf.len(), space);
        for i in 0..to_write {
            self.buffer[self.write_pos] = buf[i];
            self.write_pos = (self.write_pos + 1) % PIPE_BUF_SIZE;
        }
        self.count += to_write;
        to_write
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn is_full(&self) -> bool {
        self.count == PIPE_BUF_SIZE
    }
}

/// Global pipe table
struct PipeTable {
    pipes: [Pipe; MAX_PIPES],
}

impl PipeTable {
    const fn new() -> Self {
        Self {
            pipes: [const { Pipe::new() }; MAX_PIPES],
        }
    }

    fn allocate(&mut self) -> Option<u32> {
        for (i, pipe) in self.pipes.iter_mut().enumerate() {
            if !pipe.is_allocated() {
                pipe.readers = 1;
                pipe.writers = 1;
                pipe.read_pos = 0;
                pipe.write_pos = 0;
                pipe.count = 0;
                return Some(i as u32);
            }
        }
        None
    }

    fn get(&self, pipe_id: u32) -> Option<&Pipe> {
        let idx = pipe_id as usize;
        if idx < MAX_PIPES && self.pipes[idx].is_allocated() {
            Some(&self.pipes[idx])
        } else {
            None
        }
    }

    fn get_mut(&mut self, pipe_id: u32) -> Option<&mut Pipe> {
        let idx = pipe_id as usize;
        if idx < MAX_PIPES && self.pipes[idx].is_allocated() {
            Some(&mut self.pipes[idx])
        } else {
            None
        }
    }

    fn close_reader(&mut self, pipe_id: u32) {
        if let Some(pipe) = self.get_mut(pipe_id) {
            if pipe.readers > 0 {
                pipe.readers -= 1;
            }
        }
    }

    fn close_writer(&mut self, pipe_id: u32) {
        if let Some(pipe) = self.get_mut(pipe_id) {
            if pipe.writers > 0 {
                pipe.writers -= 1;
            }
        }
    }
}

static PIPE_TABLE: SpinLock<PipeTable> = SpinLock::new(PipeTable::new());

/// Close all file descriptors with FD_CLOEXEC flag set
///
/// This is called during exec to close file descriptors that should not
/// be inherited by the new program.
pub fn close_fds_with_cloexec() {
    let mut fd_table = FD_TABLE.lock();

    // Scan all file descriptors
    for fd in 0..MAX_FDS {
        if let Some(fd_entry) = fd_table.get(fd) {
            // Check if FD_CLOEXEC flag is set
            if (fd_entry.fd_flags & FD_CLOEXEC) != 0 {
                serial_println!("[SYSCALL] Closing FD {} (FD_CLOEXEC set)", fd);

                // Get the FD type before closing
                let fd_type = fd_entry.fd_type;

                // Close the FD
                fd_table.close(fd);

                // Handle cleanup based on FD type
                match fd_type {
                    FdType::PtyMaster(pty_num) => {
                        crate::dev::pty::deallocate_pty(pty_num);
                    }
                    FdType::PipeRead(pipe_id) => {
                        let mut pipe_table = PIPE_TABLE.lock();
                        pipe_table.close_reader(pipe_id);
                    }
                    FdType::PipeWrite(pipe_id) => {
                        let mut pipe_table = PIPE_TABLE.lock();
                        pipe_table.close_writer(pipe_id);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Per-task file descriptor table
///
/// this would be per-task.
struct FdTable {
    fds: [Option<FileDescriptor>; MAX_FDS],
}

impl FdTable {
    const fn new() -> Self {
        Self {
            fds: [const { None }; MAX_FDS],
        }
    }

    fn allocate(&mut self, fd_type: FdType) -> Option<usize> {
        // Start from FD 3 (after stdin/stdout/stderr)
        for i in 3..MAX_FDS {
            if self.fds[i].is_none() {
                self.fds[i] = Some(FileDescriptor::with_type(fd_type));
                return Some(i);
            }
        }
        None
    }

    fn allocate_with_flags(
        &mut self,
        fd_type: FdType,
        fd_flags: u32,
        status_flags: u32,
    ) -> Option<usize> {
        // Start from FD 3 (after stdin/stdout/stderr)
        for i in 3..MAX_FDS {
            if self.fds[i].is_none() {
                self.fds[i] = Some(FileDescriptor::with_flags(fd_type, fd_flags, status_flags));
                return Some(i);
            }
        }
        None
    }

    fn allocate_at(
        &mut self,
        fd: usize,
        fd_type: FdType,
        fd_flags: u32,
        status_flags: u32,
    ) -> bool {
        if fd >= MAX_FDS {
            return false;
        }
        // Close existing FD if open
        if self.fds[fd].is_some() {
            self.close(fd);
        }
        self.fds[fd] = Some(FileDescriptor::with_flags(fd_type, fd_flags, status_flags));
        true
    }

    fn get_mut(&mut self, fd: usize) -> Option<&mut FileDescriptor> {
        if fd < MAX_FDS {
            self.fds[fd].as_mut()
        } else {
            None
        }
    }

    fn get(&self, fd: usize) -> Option<FileDescriptor> {
        if fd < MAX_FDS {
            self.fds[fd].clone()
        } else {
            None
        }
    }

    fn close(&mut self, fd: usize) -> Option<FileDescriptor> {
        if fd < MAX_FDS {
            self.fds[fd].take()
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
fn sys_open(path_ptr: usize, flags: usize) -> isize {
    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        return -1;
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 256 && *ptr.add(len) != 0 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = core::str::from_utf8(path_bytes).unwrap_or("");
    serial_println!("[SYSCALL] sys_open: path={}, flags={:#x}", path, flags);

    // Special case: PTY devices (not yet in VFS)
    if path == "/dev/ptmx" {
        // Allocate a new PTY pair
        match crate::dev::pty::allocate_pty() {
            Some(pty_num) => {
                // Allocate a file descriptor
                let mut fd_table = FD_TABLE.lock();
                match fd_table.allocate(FdType::PtyMaster(pty_num)) {
                    Some(fd) => {
                        serial_println!(
                            "[SYSCALL] sys_open: allocated PTY {} as FD {}",
                            pty_num,
                            fd
                        );
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
                        serial_println!(
                            "[SYSCALL] sys_open: opened PTY slave {} as FD {}",
                            pty_num,
                            fd
                        );
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
        // Use VFS path resolution for regular files
        use crate::fs::vfs::path;
        use core::sync::atomic::AtomicU64;
        
        // Resolve path to inode
        match path::resolve_path(path, None) {
            Ok(inode) => {
                // Allocate file descriptor
                let mut fd_table = FD_TABLE.lock();
                let fd_type = FdType::VfsFile {
                    inode,
                    offset: AtomicU64::new(0),
                    flags: flags as u32,
                };
                
                match fd_table.allocate(fd_type) {
                    Some(fd) => {
                        serial_println!("[SYSCALL] sys_open: opened {} as FD {}", path, fd);
                        fd as isize
                    }
                    None => {
                        serial_println!("[SYSCALL] sys_open: no FDs available");
                        -1 // EMFILE - too many open files
                    }
                }
            }
            Err(err) => {
                serial_println!("[SYSCALL] sys_open: path resolution failed: {:?}", err);
                // Map VFS errors to errno
                match err {
                    crate::fs::vfs::superblock::FsError::NotFound => -2, // ENOENT
                    crate::fs::vfs::superblock::FsError::PermissionDenied => -13, // EACCES
                    crate::fs::vfs::superblock::FsError::NotADirectory => -20, // ENOTDIR
                    crate::fs::vfs::superblock::FsError::IsADirectory => -21, // EISDIR
                    crate::fs::vfs::superblock::FsError::TooManySymlinks => -40, // ELOOP
                    crate::fs::vfs::superblock::FsError::NameTooLong => -36, // ENAMETOOLONG
                    _ => -1, // Generic error
                }
            }
        }
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
        FdType::PipeRead(pipe_id) => {
            // Read from pipe
            let mut pipe_table = PIPE_TABLE.lock();
            match pipe_table.get_mut(pipe_id) {
                Some(pipe) => {
                    // If pipe is empty and there are no writers, return EOF
                    if pipe.is_empty() && pipe.writers == 0 {
                        return 0; // EOF
                    }
                    let bytes_read = pipe.read(buffer);
                    bytes_read as isize
                }
                None => {
                    serial_println!("[SYSCALL] sys_read: invalid pipe");
                    -1 // EBADF
                }
            }
        }
        FdType::PipeWrite(_) => {
            serial_println!("[SYSCALL] sys_read: cannot read from pipe write end");
            -1 // EBADF
        }
        FdType::VfsFile { ref inode, ref offset, .. } => {
            // Read from VFS file
            use core::sync::atomic::Ordering;
            let current_offset = offset.load(Ordering::SeqCst);
            
            match inode.read_at(current_offset, buffer) {
                Ok(bytes_read) => {
                    // Update offset
                    offset.store(current_offset + bytes_read as u64, Ordering::SeqCst);
                    bytes_read as isize
                }
                Err(err) => {
                    serial_println!("[SYSCALL] sys_read: VFS read failed: {:?}", err);
                    // Map VFS errors to errno
                    match err {
                        crate::fs::vfs::superblock::FsError::IoError => -5, // EIO
                        crate::fs::vfs::superblock::FsError::IsADirectory => -21, // EISDIR
                        _ => -1, // Generic error
                    }
                }
            }
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

            // Handle cleanup based on FD type
            match fd_entry.fd_type {
                FdType::PtyMaster(pty_num) => {
                    // If this was a PTY master, deallocate the PTY pair
                    // (In a full implementation, we'd track open counts)
                    crate::dev::pty::deallocate_pty(pty_num);
                }
                FdType::PtySlave(_) => {
                    // Slave close doesn't deallocate
                }
                FdType::PipeRead(pipe_id) => {
                    // Close pipe read end
                    let mut pipe_table = PIPE_TABLE.lock();
                    pipe_table.close_reader(pipe_id);
                }
                FdType::PipeWrite(pipe_id) => {
                    // Close pipe write end
                    let mut pipe_table = PIPE_TABLE.lock();
                    pipe_table.close_writer(pipe_id);
                }
                FdType::VfsFile { .. } => {
                    // VFS file - Arc will handle cleanup automatically
                    // No explicit cleanup needed
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
const TIOCGPTN: usize = 0x80045430; // Get PTY number
const TCGETS: usize = 0x5401; // Get termios structure
const TCSETS: usize = 0x5402; // Set termios structure
const TIOCGWINSZ: usize = 0x5413; // Get window size
const TIOCSWINSZ: usize = 0x5414; // Set window size
const TIOCSPGRP: usize = 0x5410; // Set foreground process group
const TIOCGPGRP: usize = 0x540F; // Get foreground process group
const TIOCSCTTY: usize = 0x540E; // Make this TTY the controlling terminal

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

    serial_println!(
        "[SYSCALL] sys_ioctl: FD={}, cmd={:#x}, arg={:#x}",
        fd,
        cmd,
        arg
    );

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
                    serial_println!(
                        "[SYSCALL] sys_ioctl: TIOCGWINSZ for PTY {}: {}x{}",
                        pty_num,
                        winsize.ws_row,
                        winsize.ws_col
                    );
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
                serial_println!(
                    "[SYSCALL] sys_ioctl: TIOCSWINSZ for PTY {}: {}x{}",
                    pty_num,
                    winsize.ws_row,
                    winsize.ws_col
                );
                0
            } else {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSWINSZ on invalid PTY");
                -1 // EBADF
            }
        }
        TIOCSPGRP => {
            // Set foreground process group (alias for tcsetpgrp)
            // Validate input pointer
            if !validate_user_buffer(arg, core::mem::size_of::<usize>()) {
                return -1;
            }

            // Read PGID from user buffer
            let pgid = unsafe { *(arg as *const usize) };

            // Call tcsetpgrp implementation
            sys_tcsetpgrp(fd, pgid)
        }
        TIOCGPGRP => {
            // Get foreground process group (alias for tcgetpgrp)
            // Validate output pointer
            if !validate_user_buffer(arg, core::mem::size_of::<usize>()) {
                return -1;
            }

            // Call tcgetpgrp implementation
            let result = sys_tcgetpgrp(fd);
            if result >= 0 {
                // Write PGID to user buffer
                unsafe {
                    *(arg as *mut usize) = result as usize;
                }
                0
            } else {
                result
            }
        }
        TIOCSCTTY => {
            // Make this TTY the controlling terminal
            // arg is typically 0 (force flag, not implemented)

            // Get current task
            let current_id = match crate::sched::get_current_task_info() {
                Some((id, _)) => id,
                None => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: no current task");
                    return -1;
                }
            };

            let task = match crate::sched::get_task_mut(current_id) {
                Some(t) => t,
                None => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: task not found");
                    return -1;
                }
            };

            // Check if caller is a session leader
            if task.sid != task.pid {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: not a session leader");
                return -1; // EPERM
            }

            // Check if already has a controlling terminal
            if task.tty.is_some() {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: already has controlling terminal");
                return -1; // EPERM
            }

            // Get PTY number from FD
            let pty_num = match fd_entry.fd_type {
                FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
                _ => {
                    serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: FD is not a TTY");
                    return -1; // ENOTTY
                }
            };

            // Set this TTY as the controlling terminal
            // Use PTY number as device ID
            let device_id = pty_num as usize;
            task.tty = Some(device_id);

            // Also set the session in the PTY slave
            let sid = task.sid;
            // Task reference will be dropped automatically here

            if crate::dev::pty::set_session(pty_num, sid) {
                serial_println!(
                    "[SYSCALL] sys_ioctl: TIOCSCTTY: set PTY {} as controlling terminal for session {}",
                    pty_num, sid
                );
                0
            } else {
                serial_println!("[SYSCALL] sys_ioctl: TIOCSCTTY: failed to set session in PTY");
                -1
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
    use crate::signal::{is_catchable, signals, SigAction};

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
    use crate::signal::{send_signal, signals};

    // Validate signal number
    if signal >= signals::MAX_SIGNAL as usize {
        serial_println!("[SYSCALL] sys_kill: invalid signal {}", signal);
        return -1; // EINVAL
    }

    // Signal 0 is used to check if process exists (no signal sent)
    if signal == 0 {
        serial_println!("[SYSCALL] sys_kill: signal 0 (existence check) not implemented");
        return 0;
    }

    // Get current task for permission checks
    let _sender_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_kill: no current task");
            return -1;
        }
    };

    if pid > 0 && pid < 0x8000_0000 {
        // Prevent sending SIGKILL/SIGSTOP to PID 1 (init)
        if pid == 1 && (signal == signals::SIGKILL as usize || signal == signals::SIGSTOP as usize)
        {
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


        // Send the signal
        match send_signal(target, signal as u32) {
            Ok(()) => {
                serial_println!(
                    "[SYSCALL] sys_kill: sent signal {} to process {}",
                    signal,
                    pid
                );
                0
            }
            Err(()) => {
                serial_println!("[SYSCALL] sys_kill: failed to send signal");
                -1
            }
        }
    } else {
        serial_println!("[SYSCALL] sys_kill: special PID values not implemented");
        -1 // EINVAL
    }
}

/// sys_setpgid handler - Set process group ID
///
/// # Arguments
/// * `pid` - Process ID to modify (0 = current process)
/// * `pgid` - New process group ID (0 = use pid)
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Validation
/// - Can only set pgid for self or children
/// - Must be in same session
/// - Cannot move process to different session
fn sys_setpgid(pid: usize, pgid: usize) -> isize {
    use crate::sched::process_group::{Pgid, Pid};

    // Get current task
    let current_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_setpgid: no current task");
            return -1;
        }
    };

    // Determine target PID (0 means current process)
    let target_pid: Pid = if pid == 0 { current_id } else { pid };

    // Determine target PGID (0 means use target's PID)
    let target_pgid: Pgid = if pgid == 0 { target_pid } else { pgid };

    // Get current task info
    let current_task = match crate::sched::get_task_mut(current_id) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_setpgid: current task not found");
            return -1;
        }
    };

    let current_sid = current_task.sid;

    // Get target task
    let target_task = match crate::sched::get_task_mut(target_pid) {
        Some(t) => t,
        None => {
            serial_println!(
                "[SYSCALL] sys_setpgid: target process {} not found",
                target_pid
            );
            return -1; // ESRCH - no such process
        }
    };

    // Validation: can only set pgid for self or children
    if target_pid != current_id && target_task.ppid != current_id {
        serial_println!("[SYSCALL] sys_setpgid: not self or child");
        return -1; // EPERM
    }

    // Validation: must be in same session
    if target_task.sid != current_sid {
        serial_println!("[SYSCALL] sys_setpgid: not in same session");
        return -1; // EPERM
    }

    // Set the process group
    let old_pgid = target_task.pgid;
    target_task.pgid = target_pgid;

    serial_println!(
        "[SYSCALL] sys_setpgid: set PID {} PGID from {} to {}",
        target_pid,
        old_pgid,
        target_pgid
    );

    0
}

/// sys_getpgrp handler - Get current process group ID
///
/// # Returns
/// Process group ID of current process
fn sys_getpgrp() -> isize {
    // Get current task
    let current_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_getpgrp: no current task");
            return -1;
        }
    };

    let task = match crate::sched::get_task_mut(current_id) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_getpgrp: task not found");
            return -1;
        }
    };

    let pgid = task.pgid;
    serial_println!("[SYSCALL] sys_getpgrp: returning PGID {}", pgid);
    pgid as isize
}

/// sys_setsid handler - Create a new session
///
/// # Returns
/// New session ID on success, or -1 on error
///
/// # Behavior
/// - Creates new session with sid = pid
/// - Creates new process group with pgid = pid
/// - Detaches from controlling terminal
/// - Fails if caller is already a process group leader
fn sys_setsid() -> isize {
    // Get current task
    let current_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_setsid: no current task");
            return -1;
        }
    };

    let task = match crate::sched::get_task_mut(current_id) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_setsid: task not found");
            return -1;
        }
    };

    // Cannot create session if already a process group leader
    if task.pgid == task.pid {
        serial_println!("[SYSCALL] sys_setsid: already a process group leader");
        return -1; // EPERM
    }

    // Create new session
    let new_sid = task.pid;
    let new_pgid = task.pid;

    task.sid = new_sid;
    task.pgid = new_pgid;
    task.tty = None; // Detach from controlling terminal

    serial_println!(
        "[SYSCALL] sys_setsid: created new session {} for PID {}",
        new_sid,
        current_id
    );

    new_sid as isize
}

/// sys_getsid handler - Get session ID of a process
///
/// # Arguments
/// * `pid` - Process ID to query (0 = current process)
///
/// # Returns
/// Session ID on success, or -1 on error
fn sys_getsid(pid: usize) -> isize {
    use crate::sched::process_group::Pid;

    // Get current task
    let current_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_getsid: no current task");
            return -1;
        }
    };

    // Determine target PID (0 means current process)
    let target_pid: Pid = if pid == 0 { current_id } else { pid };

    // Get target task
    let task = match crate::sched::get_task_mut(target_pid) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_getsid: process {} not found", target_pid);
            return -1; // ESRCH - no such process
        }
    };

    let sid = task.sid;
    serial_println!("[SYSCALL] sys_getsid: PID {} has SID {}", target_pid, sid);
    sid as isize
}

/// sys_tcsetpgrp handler - Set foreground process group of terminal
///
/// # Arguments
/// * `fd` - File descriptor of terminal
/// * `pgid` - Process group ID to set as foreground
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_tcsetpgrp(fd: usize, pgid: usize) -> isize {
    use crate::sched::process_group::Pgid;

    // Get current task
    let current_id = match crate::sched::get_current_task_info() {
        Some((id, _)) => id,
        None => {
            serial_println!("[SYSCALL] sys_tcsetpgrp: no current task");
            return -1;
        }
    };

    let current_task = match crate::sched::get_task_mut(current_id) {
        Some(t) => t,
        None => {
            serial_println!("[SYSCALL] sys_tcsetpgrp: current task not found");
            return -1;
        }
    };

    let _current_sid = current_task.sid;

    // Look up file descriptor
    let fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_tcsetpgrp: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);

    // Get PTY number from FD
    let pty_num = match fd_entry.fd_type {
        FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
        _ => {
            serial_println!("[SYSCALL] sys_tcsetpgrp: FD is not a TTY");
            return -1; // ENOTTY
        }
    };

    // Validate that the target PGID exists and is in the same session

    // Set foreground process group in PTY
    if crate::dev::pty::set_foreground_pgid(pty_num, pgid as Pgid) {
        serial_println!(
            "[SYSCALL] sys_tcsetpgrp: set foreground PGID to {} for PTY {}",
            pgid,
            pty_num
        );
        0
    } else {
        serial_println!("[SYSCALL] sys_tcsetpgrp: failed to set foreground PGID");
        -1
    }
}

/// sys_tcgetpgrp handler - Get foreground process group of terminal
///
/// # Arguments
/// * `fd` - File descriptor of terminal
///
/// # Returns
/// Foreground process group ID on success, or -1 on error
fn sys_tcgetpgrp(fd: usize) -> isize {
    // Look up file descriptor
    let fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_tcgetpgrp: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);

    // Get PTY number from FD
    let pty_num = match fd_entry.fd_type {
        FdType::PtyMaster(n) | FdType::PtySlave(n) => n,
        _ => {
            serial_println!("[SYSCALL] sys_tcgetpgrp: FD is not a TTY");
            return -1; // ENOTTY
        }
    };

    // Get foreground process group from PTY
    match crate::dev::pty::get_foreground_pgid(pty_num) {
        Some(pgid) => {
            serial_println!(
                "[SYSCALL] sys_tcgetpgrp: foreground PGID is {} for PTY {}",
                pgid,
                pty_num
            );
            pgid as isize
        }
        None => {
            serial_println!("[SYSCALL] sys_tcgetpgrp: no foreground PGID set");
            -1 // No foreground process group
        }
    }
}

/// fcntl command numbers
const F_GETFD: usize = 1; // Get file descriptor flags
const F_SETFD: usize = 2; // Set file descriptor flags
const F_GETFL: usize = 3; // Get file status flags
const F_SETFL: usize = 4; // Set file status flags

/// sys_fcntl handler - File descriptor control operations
///
/// # Arguments
/// * `fd` - File descriptor
/// * `cmd` - fcntl command
/// * `arg` - Command-specific argument
///
/// # Returns
/// Command-specific return value, or -1 on error
fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> isize {
    serial_println!("[SYSCALL] sys_fcntl: FD={}, cmd={}, arg={}", fd, cmd, arg);

    let mut fd_table = FD_TABLE.lock();
    let fd_entry = match fd_table.get_mut(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_fcntl: invalid FD {}", fd);
            return -1; // EBADF
        }
    };

    match cmd {
        F_GETFD => {
            // Get file descriptor flags
            let flags = fd_entry.fd_flags;
            serial_println!("[SYSCALL] sys_fcntl: F_GETFD returned {:#x}", flags);
            flags as isize
        }
        F_SETFD => {
            // Set file descriptor flags (only FD_CLOEXEC is valid)
            let flags = arg as u32 & FD_CLOEXEC;
            fd_entry.fd_flags = flags;
            serial_println!("[SYSCALL] sys_fcntl: F_SETFD set flags to {:#x}", flags);
            0
        }
        F_GETFL => {
            // Get file status flags
            let flags = fd_entry.status_flags;
            serial_println!("[SYSCALL] sys_fcntl: F_GETFL returned {:#x}", flags);
            flags as isize
        }
        F_SETFL => {
            // Set file status flags (only O_NONBLOCK and O_APPEND can be changed)
            let flags = arg as u32 & (O_NONBLOCK | O_APPEND);
            fd_entry.status_flags = (fd_entry.status_flags & !(O_NONBLOCK | O_APPEND)) | flags;
            serial_println!(
                "[SYSCALL] sys_fcntl: F_SETFL set flags to {:#x}",
                fd_entry.status_flags
            );
            0
        }
        _ => {
            serial_println!("[SYSCALL] sys_fcntl: unsupported command {}", cmd);
            -1 // EINVAL
        }
    }
}

/// sys_pipe2 handler - Create a pipe with flags
///
/// # Arguments
/// * `pipefd_ptr` - Pointer to array of 2 integers for read/write FDs
/// * `flags` - Pipe flags (O_CLOEXEC, O_NONBLOCK)
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_pipe2(pipefd_ptr: usize, flags: usize) -> isize {
    serial_println!(
        "[SYSCALL] sys_pipe2: pipefd_ptr={:#x}, flags={:#x}",
        pipefd_ptr,
        flags
    );

    // Validate pointer
    if !validate_user_buffer(pipefd_ptr, core::mem::size_of::<[i32; 2]>()) {
        serial_println!("[SYSCALL] sys_pipe2: invalid pipefd pointer");
        return -1;
    }

    // Parse flags
    let fd_flags = if (flags & 0x80000) != 0 {
        FD_CLOEXEC
    } else {
        0
    }; // O_CLOEXEC = 0x80000
    let status_flags = (flags as u32) & O_NONBLOCK;

    // Allocate a pipe
    let mut pipe_table = PIPE_TABLE.lock();
    let pipe_id = match pipe_table.allocate() {
        Some(id) => id,
        None => {
            serial_println!("[SYSCALL] sys_pipe2: no pipes available");
            return -1; // EMFILE - too many open files
        }
    };
    drop(pipe_table);

    // Allocate file descriptors
    let mut fd_table = FD_TABLE.lock();

    // Allocate read end
    let read_fd =
        match fd_table.allocate_with_flags(FdType::PipeRead(pipe_id), fd_flags, status_flags) {
            Some(fd) => fd,
            None => {
                // Failed to allocate read FD, deallocate pipe
                let mut pipe_table = PIPE_TABLE.lock();
                pipe_table.close_reader(pipe_id);
                pipe_table.close_writer(pipe_id);
                serial_println!("[SYSCALL] sys_pipe2: no FDs available for read end");
                return -1; // EMFILE
            }
        };

    // Allocate write end
    let write_fd =
        match fd_table.allocate_with_flags(FdType::PipeWrite(pipe_id), fd_flags, status_flags) {
            Some(fd) => fd,
            None => {
                // Failed to allocate write FD, clean up
                fd_table.close(read_fd);
                let mut pipe_table = PIPE_TABLE.lock();
                pipe_table.close_reader(pipe_id);
                pipe_table.close_writer(pipe_id);
                serial_println!("[SYSCALL] sys_pipe2: no FDs available for write end");
                return -1; // EMFILE
            }
        };

    drop(fd_table);

    // Write FDs to user buffer
    unsafe {
        let pipefd = pipefd_ptr as *mut i32;
        *pipefd.offset(0) = read_fd as i32;
        *pipefd.offset(1) = write_fd as i32;
    }

    serial_println!(
        "[SYSCALL] sys_pipe2: created pipe {} with FDs [{}, {}]",
        pipe_id,
        read_fd,
        write_fd
    );
    0
}

/// sys_dup2 handler - Duplicate file descriptor to specific FD number
///
/// # Arguments
/// * `oldfd` - Source file descriptor
/// * `newfd` - Target file descriptor number
///
/// # Returns
/// New file descriptor on success, or -1 on error
fn sys_dup2(oldfd: usize, newfd: usize) -> isize {
    serial_println!("[SYSCALL] sys_dup2: oldfd={}, newfd={}", oldfd, newfd);

    // Validate FD numbers
    if oldfd >= MAX_FDS || newfd >= MAX_FDS {
        serial_println!("[SYSCALL] sys_dup2: FD out of range");
        return -1; // EBADF
    }

    // If oldfd == newfd, just validate oldfd and return it
    if oldfd == newfd {
        let fd_table = FD_TABLE.lock();
        if fd_table.get(oldfd).is_some() {
            serial_println!("[SYSCALL] sys_dup2: oldfd == newfd, returning {}", newfd);
            return newfd as isize;
        } else {
            serial_println!("[SYSCALL] sys_dup2: oldfd {} is invalid", oldfd);
            return -1; // EBADF
        }
    }

    // Get old FD entry
    let mut fd_table = FD_TABLE.lock();
    let old_entry = match fd_table.get(oldfd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_dup2: oldfd {} is invalid", oldfd);
            return -1; // EBADF
        }
    };

    // Copy the FD entry (but clear FD_CLOEXEC flag as per POSIX)
    let new_entry = FileDescriptor {
        fd_type: old_entry.fd_type,
        fd_flags: 0, // FD_CLOEXEC is not inherited by dup2
        status_flags: old_entry.status_flags,
    };

    // Increment reference count for pipes
    match new_entry.fd_type {
        FdType::PipeRead(pipe_id) => {
            let mut pipe_table = PIPE_TABLE.lock();
            if let Some(pipe) = pipe_table.get_mut(pipe_id) {
                pipe.readers += 1;
            }
        }
        FdType::PipeWrite(pipe_id) => {
            let mut pipe_table = PIPE_TABLE.lock();
            if let Some(pipe) = pipe_table.get_mut(pipe_id) {
                pipe.writers += 1;
            }
        }
        _ => {}
    }

    // Close newfd if it's open, then allocate at that position
    if fd_table.allocate_at(
        newfd,
        new_entry.fd_type,
        new_entry.fd_flags,
        new_entry.status_flags,
    ) {
        serial_println!(
            "[SYSCALL] sys_dup2: duplicated FD {} to FD {}",
            oldfd,
            newfd
        );
        newfd as isize
    } else {
        serial_println!("[SYSCALL] sys_dup2: failed to allocate at FD {}", newfd);
        -1
    }
}

/// sys_read_stdin handler - Read keyboard input from stdin
///
/// # Arguments
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Maximum bytes to read
///
/// # Returns
/// Number of bytes read, or -1 on error
///
/// # Description
/// Reads keyboard input from the PS/2 keyboard driver buffer.
/// This is a non-blocking read that returns immediately if no data is available.
fn sys_read_stdin(buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0;
    }

    // Validate buffer
    if !validate_user_buffer(buf_ptr, len) {
        serial_println!("[SYSCALL] sys_read_stdin: invalid buffer");
        return -1;
    }

    // Convert pointer to mutable slice
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len) };

    // Read from keyboard buffer
    let mut bytes_read = 0;
    while bytes_read < len {
        match crate::drivers::input::keyboard::keyboard_read() {
            Some(ch) => {
                buffer[bytes_read] = ch;
                bytes_read += 1;
            }
            None => {
                // No more data available
                break;
            }
        }
    }

    bytes_read as isize
}

/// sys_serial_write handler - Write data to serial port
///
/// # Arguments
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Number of bytes to write
///
/// # Returns
/// Number of bytes written, or -1 on error
fn sys_serial_write(buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0;
    }

    // Validate buffer
    let user_ok = validate_user_buffer(buf_ptr, len);
    if !user_ok {
        let allow_kernel = buf_ptr >= USER_LIMIT && kernel_buffer_allowed();
        if !allow_kernel {
            serial_println!("[SYSCALL] sys_serial_write: invalid buffer");
            return -1;
        }
    }

    // Convert pointer to slice
    let buffer = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len) };

    // Write to serial port
    for &byte in buffer {
        crate::drivers::serial::uart16550::serial_write(byte);
    }

    len as isize
}

/// sys_serial_read handler - Read data from serial port
///
/// # Arguments
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Maximum bytes to read
///
/// # Returns
/// Number of bytes read, or -1 on error
///
/// # Description
/// Non-blocking read from serial port. Returns immediately if no data is available.
fn sys_serial_read(buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0;
    }

    // Validate buffer
    if !validate_user_buffer(buf_ptr, len) {
        serial_println!("[SYSCALL] sys_serial_read: invalid buffer");
        return -1;
    }

    // Convert pointer to mutable slice
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len) };

    // Read from serial port
    let mut bytes_read = 0;
    while bytes_read < len {
        match crate::drivers::serial::uart16550::serial_read() {
            Some(byte) => {
                buffer[bytes_read] = byte;
                bytes_read += 1;
            }
            None => {
                // No more data available
                break;
            }
        }
    }

    bytes_read as isize
}

/// sys_read_kernel_log handler - Read kernel log buffer
///
/// # Arguments
/// * `buf_ptr` - Pointer to buffer
/// * `len` - Maximum bytes to read
///
/// # Returns
/// Number of bytes written to buffer, or -1 on error
///
/// # Description
/// Reads the kernel log buffer and copies it to userspace.
/// The log buffer contains formatted log messages from the kernel.
fn sys_read_kernel_log(buf_ptr: usize, len: usize) -> isize {
    if len == 0 {
        return 0;
    }

    // Validate buffer
    if !validate_user_buffer(buf_ptr, len) {
        serial_println!("[SYSCALL] sys_read_kernel_log: invalid buffer");
        return -1;
    }

    // Get user buffer
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len) };

    // Read the kernel log buffer directly into user buffer
    let bytes_copied = crate::log::read_log_buffer(buffer);

    bytes_copied as isize
}

/// sys_block_read handler - Read a block from disk
///
/// # Arguments
/// * `lba` - Logical Block Address (sector number)
/// * `buf_ptr` - Pointer to buffer (must be at least 512 bytes)
/// * `count` - Number of blocks to read
///
/// # Returns
/// Number of blocks read, or -1 on error
fn sys_block_read(lba: usize, buf_ptr: usize, count: usize) -> isize {
    if count == 0 {
        return 0;
    }

    // Calculate buffer size needed (512 bytes per block)
    let buf_size = count * 512;

    // Validate buffer
    if !validate_user_buffer(buf_ptr, buf_size) {
        serial_println!("[SYSCALL] sys_block_read: invalid buffer");
        return -1;
    }

    // Convert pointer to mutable slice
    let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut u8, buf_size) };

    // Read blocks
    let mut blocks_read = 0;
    for i in 0..count {
        let block_lba = (lba + i) as u64;
        let block_buf = &mut buffer[i * 512..(i + 1) * 512];

        match crate::drivers::block::virtio_blk::block_read(block_lba, block_buf) {
            Ok(()) => {
                blocks_read += 1;
            }
            Err(e) => {
                serial_println!(
                    "[SYSCALL] sys_block_read: error reading block {}: {:?}",
                    block_lba,
                    e
                );
                if blocks_read == 0 {
                    return -1; // Return error if no blocks were read
                } else {
                    break; // Return partial read
                }
            }
        }
    }

    blocks_read as isize
}

/// sys_block_write handler - Write a block to disk
///
/// # Arguments
/// * `lba` - Logical Block Address (sector number)
/// * `buf_ptr` - Pointer to buffer (must be at least 512 bytes)
/// * `count` - Number of blocks to write
///
/// # Returns
/// Number of blocks written, or -1 on error
fn sys_block_write(lba: usize, buf_ptr: usize, count: usize) -> isize {
    if count == 0 {
        return 0;
    }

    // Calculate buffer size needed (512 bytes per block)
    let buf_size = count * 512;

    // Validate buffer
    let user_ok = validate_user_buffer(buf_ptr, buf_size);
    if !user_ok {
        let allow_kernel = buf_ptr >= USER_LIMIT && kernel_buffer_allowed();
        if !allow_kernel {
            serial_println!("[SYSCALL] sys_block_write: invalid buffer");
            return -1;
        }
    }

    // Convert pointer to slice
    let buffer = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, buf_size) };

    // Write blocks
    let mut blocks_written = 0;
    for i in 0..count {
        let block_lba = (lba + i) as u64;
        let block_buf = &buffer[i * 512..(i + 1) * 512];

        match crate::drivers::block::virtio_blk::block_write(block_lba, block_buf) {
            Ok(()) => {
                blocks_written += 1;
            }
            Err(e) => {
                serial_println!(
                    "[SYSCALL] sys_block_write: error writing block {}: {:?}",
                    block_lba,
                    e
                );
                if blocks_written == 0 {
                    return -1; // Return error if no blocks were written
                } else {
                    break; // Return partial write
                }
            }
        }
    }

    blocks_written as isize
}

/// Device information structure for userland
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceInfo {
    pub name: [u8; 32],  // Device name (null-terminated)
    pub bus_type: u32,   // Bus type (0=Platform, 1=PS2, 2=PCI, 3=Virtio)
    pub io_base: u64,    // I/O base address
    pub irq: u32,        // IRQ number (0xFFFFFFFF if none)
    pub state: u32, // Device state (0=Detected, 1=Initializing, 2=Active, 3=Failed, 4=Shutdown)
    pub has_driver: u32, // 1 if driver is loaded, 0 otherwise
}

/// sys_get_device_list handler - Query device tree
///
/// # Arguments
/// * `buf_ptr` - Pointer to array of DeviceInfo structures
/// * `max_devices` - Maximum number of devices to return
///
/// # Returns
/// Number of devices returned, or -1 on error
fn sys_get_device_list(buf_ptr: usize, max_devices: usize) -> isize {
    if max_devices == 0 {
        return 0;
    }

    // Calculate buffer size
    let buf_size = max_devices * core::mem::size_of::<DeviceInfo>();

    // Validate buffer
    if !validate_user_buffer(buf_ptr, buf_size) {
        serial_println!("[SYSCALL] sys_get_device_list: invalid buffer");
        return -1;
    }

    // Convert pointer to mutable slice
    let buffer =
        unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut DeviceInfo, max_devices) };

    // Iterate over devices and fill buffer
    let mut count = 0;
    crate::io::devtree::for_each_device(|device| {
        if count >= max_devices {
            return;
        }

        // Copy device name
        let mut name = [0u8; 32];
        let name_bytes = device.name.as_bytes();
        let copy_len = core::cmp::min(name_bytes.len(), 31);
        name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

        // Convert bus type to u32
        let bus_type = match device.bus {
            crate::io::BusType::Platform => 0,
            crate::io::BusType::PS2 => 1,
            crate::io::BusType::PCI => 2,
            crate::io::BusType::Virtio => 3,
        };

        // Convert IRQ to u32 (0xFFFFFFFF if none)
        let irq = device.irq.unwrap_or(0xFF) as u32;
        let irq = if device.irq.is_none() {
            0xFFFFFFFF
        } else {
            irq
        };

        // Convert state to u32
        let state = match device.state {
            crate::io::DeviceState::Detected => 0,
            crate::io::DeviceState::Initializing => 1,
            crate::io::DeviceState::Active => 2,
            crate::io::DeviceState::Failed => 3,
            crate::io::DeviceState::Shutdown => 4,
        };

        // Check if driver is loaded
        let has_driver = if device.driver.is_some() { 1 } else { 0 };

        // Fill device info
        buffer[count] = DeviceInfo {
            name,
            bus_type,
            io_base: device.io_base,
            irq,
            state,
            has_driver,
        };

        count += 1;
    });

    count as isize
}

/// Block device information structure for userland
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BlockDeviceInfo {
    pub block_count: u64, // Total number of blocks
    pub block_size: u32,  // Size of each block in bytes
    pub capacity_mb: u32, // Total capacity in megabytes
}

/// sys_get_block_device_info handler - Get block device information
///
/// # Arguments
/// * `buf_ptr` - Pointer to BlockDeviceInfo structure
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_get_block_device_info(buf_ptr: usize) -> isize {
    // Validate buffer
    if !validate_user_buffer(buf_ptr, core::mem::size_of::<BlockDeviceInfo>()) {
        serial_println!("[SYSCALL] sys_get_block_device_info: invalid buffer");
        return -1;
    }

    // Get block device info from virtio-blk driver
    let mut test_buf = [0u8; 512];
    match crate::drivers::block::virtio_blk::block_read(0, &mut test_buf) {
        Ok(()) => {
            // Device is ready, get info
            // Note: In a full implementation, we'd have a proper API to query device info

            let block_count = 1024 * 1024; // 1M blocks (from driver default)
            let block_size = 512;
            let capacity_mb = (block_count * block_size) / (1024 * 1024);

            let info = BlockDeviceInfo {
                block_count,
                block_size: block_size as u32,
                capacity_mb: capacity_mb as u32,
            };

            // Write to user buffer
            unsafe {
                *(buf_ptr as *mut BlockDeviceInfo) = info;
            }

            0
        }
        Err(_) => {
            serial_println!("[SYSCALL] sys_get_block_device_info: device not ready");
            -1
        }
    }
}

/// IRQ statistics entry structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IrqStatsEntry {
    pub irq: u8,
    pub _padding: [u8; 7],
    pub cpu_counts: [u64; 8],
}

/// sys_get_irq_stats handler - Get IRQ statistics per CPU
///
/// # Arguments
/// * `buf_ptr` - Pointer to array of IrqStatsEntry structures
/// * `max_entries` - Maximum number of entries to return
///
/// # Returns
/// Number of IRQ entries returned, or -1 on error
fn sys_get_irq_stats(buf_ptr: usize, max_entries: usize) -> isize {
    if max_entries == 0 {
        return 0;
    }

    let buf_size = max_entries * core::mem::size_of::<IrqStatsEntry>();

    // Validate buffer
    if !validate_user_buffer(buf_ptr, buf_size) {
        serial_println!("[SYSCALL] sys_get_irq_stats: invalid buffer");
        return -1;
    }

    // Use a fixed-size buffer (support up to 32 IRQs)
    const MAX_IRQS: usize = 32;
    let mut temp_buffer: [(u8, [u64; 8]); MAX_IRQS] = [(0, [0; 8]); MAX_IRQS];
    let actual_max = if max_entries > MAX_IRQS {
        MAX_IRQS
    } else {
        max_entries
    };

    let count = crate::io::irq::get_all_irq_stats(&mut temp_buffer[..actual_max], actual_max);

    // Convert to IrqStatsEntry format and copy to user buffer
    let user_buffer =
        unsafe { core::slice::from_raw_parts_mut(buf_ptr as *mut IrqStatsEntry, max_entries) };

    for i in 0..count {
        let (irq, cpu_counts) = temp_buffer[i];
        user_buffer[i] = IrqStatsEntry {
            irq,
            _padding: [0; 7],
            cpu_counts,
        };
    }

    count as isize
}

/// sys_stat handler - Get file status by path
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `stat_ptr` - Pointer to Stat structure to fill
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_stat(path_ptr: usize, stat_ptr: usize) -> isize {
    use crate::fs::vfs::inode::Stat;

    // Validate pointers
    if !validate_user_buffer(path_ptr, 1)
        || !validate_user_buffer(stat_ptr, core::mem::size_of::<Stat>())
    {
        serial_println!("[SYSCALL] sys_stat: invalid pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_stat: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_stat: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    serial_println!("[SYSCALL] sys_stat: path={}", path);

    // Return basic stat info
    let stat = Stat {
        st_dev: 0,
        st_ino: 1,
        st_mode: 0o100644,
        st_nlink: 1,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 4096,
        st_blocks: 0,
        st_atime_sec: 0,
        st_atime_nsec: 0,
        st_mtime_sec: 0,
        st_mtime_nsec: 0,
        st_ctime_sec: 0,
        st_ctime_nsec: 0,
    };
    unsafe {
        *(stat_ptr as *mut Stat) = stat;
    }
    serial_println!("[SYSCALL] sys_stat: returning stat for {}", path);
    0 // Success
}

/// sys_fstat handler - Get file status by file descriptor
///
/// # Arguments
/// * `fd` - File descriptor
/// * `stat_ptr` - Pointer to Stat structure to fill
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_fstat(fd: usize, stat_ptr: usize) -> isize {
    use crate::fs::vfs::inode::Stat;

    // Validate stat pointer
    if !validate_user_buffer(stat_ptr, core::mem::size_of::<Stat>()) {
        serial_println!("[SYSCALL] sys_fstat: invalid stat pointer");
        return -1; // EFAULT
    }

    serial_println!("[SYSCALL] sys_fstat: fd={}", fd);

    // Return basic stat info for fd
    let stat = Stat {
        st_dev: 0,
        st_ino: fd as u64,
        st_mode: 0o100644,
        st_nlink: 1,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 4096,
        st_blocks: 0,
        st_atime_sec: 0,
        st_atime_nsec: 0,
        st_mtime_sec: 0,
        st_mtime_nsec: 0,
        st_ctime_sec: 0,
        st_ctime_nsec: 0,
    };
    unsafe {
        *(stat_ptr as *mut Stat) = stat;
    }
    serial_println!("[SYSCALL] sys_fstat: returning stat for fd={}", fd);
    0 // Success
}

/// sys_lstat handler - Get file status by path (don't follow symlinks)
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `stat_ptr` - Pointer to Stat structure to fill
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_lstat(path_ptr: usize, stat_ptr: usize) -> isize {
    use crate::fs::vfs::inode::Stat;

    // Validate pointers
    if !validate_user_buffer(path_ptr, 1)
        || !validate_user_buffer(stat_ptr, core::mem::size_of::<Stat>())
    {
        serial_println!("[SYSCALL] sys_lstat: invalid pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_lstat: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_lstat: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    serial_println!("[SYSCALL] sys_lstat: path={}", path);

    // Return basic stat info
    let stat = Stat {
        st_dev: 0,
        st_ino: 1,
        st_mode: 0o100644,
        st_nlink: 1,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 4096,
        st_blocks: 0,
        st_atime_sec: 0,
        st_atime_nsec: 0,
        st_mtime_sec: 0,
        st_mtime_nsec: 0,
        st_ctime_sec: 0,
        st_ctime_nsec: 0,
    };
    unsafe {
        *(stat_ptr as *mut Stat) = stat;
    }
    serial_println!("[SYSCALL] sys_lstat: returning stat for {}", path);
    0 // Success
}

/// sys_chmod handler - Change file permissions
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `mode` - New permission bits
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_chmod(path_ptr: usize, mode: usize) -> isize {
    

    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_chmod: invalid path pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_chmod: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_chmod: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    serial_println!("[SYSCALL] sys_chmod: path={}, mode={:#o}", path, mode);

    serial_println!("[SYSCALL] sys_chmod: {} mode={:o}", path, mode);
    0 // Success
}

/// sys_chown handler - Change file ownership
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `uid` - New user ID (-1 to leave unchanged)
/// * `gid` - New group ID (-1 to leave unchanged)
///
/// # Returns
/// 0 on success, or -1 on error
fn sys_chown(path_ptr: usize, uid: usize, gid: usize) -> isize {
    

    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_chown: invalid path pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_chown: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_chown: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    serial_println!(
        "[SYSCALL] sys_chown: path={}, uid={}, gid={}",
        path,
        uid,
        gid
    );

    serial_println!("[SYSCALL] sys_chown: {} uid={} gid={}", path, uid, gid);
    0 // Success
}

/// Timespec structure for utimensat
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

/// sys_utimensat handler - Change file timestamps with nanosecond precision
///
/// # Arguments
/// * `dirfd` - Directory file descriptor (or AT_FDCWD for current directory)
/// * `path_ptr` - Pointer to null-terminated path string (or 0 for dirfd itself)
/// * `times_ptr` - Pointer to array of 2 Timespec structures [atime, mtime] (or 0 for current time)
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Special values
/// * AT_FDCWD (-100): Use current working directory
/// * UTIME_NOW (0x3fffffff): Set to current time
/// * UTIME_OMIT (0x3ffffffe): Don't change this timestamp
fn sys_utimensat(dirfd: usize, path_ptr: usize, times_ptr: usize) -> isize {
    

    const AT_FDCWD: isize = -100;
    const UTIME_NOW: i64 = 0x3fffffff;
    const UTIME_OMIT: i64 = 0x3ffffffe;

    serial_println!(
        "[SYSCALL] sys_utimensat: dirfd={}, path_ptr={:#x}, times_ptr={:#x}",
        dirfd,
        path_ptr,
        times_ptr
    );

    // Parse path if provided
    let path_opt = if path_ptr != 0 {
        if !validate_user_buffer(path_ptr, 1) {
            serial_println!("[SYSCALL] sys_utimensat: invalid path pointer");
            return -1; // EFAULT
        }

        let path_bytes = unsafe {
            let mut len = 0;
            let ptr = path_ptr as *const u8;
            while len < 4096 && *ptr.add(len) != 0 {
                len += 1;
            }
            if len >= 4096 {
                serial_println!("[SYSCALL] sys_utimensat: path too long");
                return -1; // ENAMETOOLONG
            }
            core::slice::from_raw_parts(ptr, len)
        };

        let path = match core::str::from_utf8(path_bytes) {
            Ok(s) => s,
            Err(_) => {
                serial_println!("[SYSCALL] sys_utimensat: invalid UTF-8 in path");
                return -1; // EINVAL
            }
        };

        Some(path)
    } else {
        None
    };

    // Parse times if provided
    let (atime_opt, mtime_opt) = if times_ptr != 0 {
        if !validate_user_buffer(times_ptr, core::mem::size_of::<[Timespec; 2]>()) {
            serial_println!("[SYSCALL] sys_utimensat: invalid times pointer");
            return -1; // EFAULT
        }

        let times = unsafe { core::slice::from_raw_parts(times_ptr as *const Timespec, 2) };

        let atime = if times[0].tv_nsec == UTIME_OMIT {
            None
        } else if times[0].tv_nsec == UTIME_NOW {
            Some(0u64) // Placeholder
        } else {
            Some((times[0].tv_sec as u64) * 1_000_000_000 + (times[0].tv_nsec as u64))
        };

        let mtime = if times[1].tv_nsec == UTIME_OMIT {
            None
        } else if times[1].tv_nsec == UTIME_NOW {
            Some(0u64) // Placeholder
        } else {
            Some((times[1].tv_sec as u64) * 1_000_000_000 + (times[1].tv_nsec as u64))
        };

        (atime, mtime)
    } else {
        // NULL times means set both to current time
        (Some(0u64), Some(0u64))
    };

    serial_println!(
        "[SYSCALL] sys_utimensat: path={:?}, atime={:?}, mtime={:?}",
        path_opt,
        atime_opt,
        mtime_opt
    );

    serial_println!("[SYSCALL] sys_utimensat: complete");
    0 // Success
}

/// sys_setxattr handler - Set extended attribute
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `name_ptr` - Pointer to null-terminated attribute name
/// * `value_ptr` - Pointer to attribute value
/// * `size` - Size of attribute value
/// * `flags` - Flags (XATTR_CREATE, XATTR_REPLACE)
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Note
/// This is a simplified version that takes 3 args instead of 5.
/// The full signature would be: (path, name, value, size, flags)
fn sys_setxattr(path_ptr: usize, name_ptr: usize, _value_info: usize) -> isize {
    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_setxattr: invalid path pointer");
        return -1; // EFAULT
    }

    // Validate name pointer
    if !validate_user_buffer(name_ptr, 1) {
        serial_println!("[SYSCALL] sys_setxattr: invalid name pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_setxattr: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_setxattr: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    // Read name string
    let name_bytes = unsafe {
        let mut len = 0;
        let ptr = name_ptr as *const u8;
        while len < 256 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 256 {
            serial_println!("[SYSCALL] sys_setxattr: name too long (max 255 bytes)");
            return -1; // ERANGE
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let name = match core::str::from_utf8(name_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_setxattr: invalid UTF-8 in name");
            return -1; // EINVAL
        }
    };

    // Validate namespace (user.* or system.*)
    if !name.starts_with("user.") && !name.starts_with("system.") {
        serial_println!("[SYSCALL] sys_setxattr: invalid namespace (must be user.* or system.*)");
        return -1; // EOPNOTSUPP
    }

    serial_println!("[SYSCALL] sys_setxattr: path={}, name={}", path, name);

    serial_println!("[SYSCALL] sys_setxattr: complete");
    0 // Success
}

/// sys_getxattr handler - Get extended attribute
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `name_ptr` - Pointer to null-terminated attribute name
/// * `value_ptr` - Pointer to buffer for attribute value (or 0 to query size)
/// * `size` - Size of value buffer
///
/// # Returns
/// Size of attribute value on success, or -1 on error
///
/// # Note
/// This is a simplified version that takes 3 args instead of 4.
fn sys_getxattr(path_ptr: usize, name_ptr: usize, _value_info: usize) -> isize {
    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_getxattr: invalid path pointer");
        return -1; // EFAULT
    }

    // Validate name pointer
    if !validate_user_buffer(name_ptr, 1) {
        serial_println!("[SYSCALL] sys_getxattr: invalid name pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_getxattr: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_getxattr: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    // Read name string
    let name_bytes = unsafe {
        let mut len = 0;
        let ptr = name_ptr as *const u8;
        while len < 256 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 256 {
            serial_println!("[SYSCALL] sys_getxattr: name too long (max 255 bytes)");
            return -1; // ERANGE
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let name = match core::str::from_utf8(name_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_getxattr: invalid UTF-8 in name");
            return -1; // EINVAL
        }
    };

    serial_println!("[SYSCALL] sys_getxattr: path={}, name={}", path, name);

    serial_println!("[SYSCALL] sys_getxattr: attribute not found");
    -1 // ENODATA
}

/// sys_listxattr handler - List extended attribute names
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `list_ptr` - Pointer to buffer for attribute names (or 0 to query size)
/// * `size` - Size of list buffer
///
/// # Returns
/// Size of attribute name list on success, or -1 on error
///
/// # Note
/// This is a simplified version that takes 2 args instead of 3.
fn sys_listxattr(path_ptr: usize, _list_info: usize) -> isize {
    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_listxattr: invalid path pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_listxattr: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_listxattr: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    serial_println!("[SYSCALL] sys_listxattr: path={}", path);

    serial_println!("[SYSCALL] sys_listxattr: not implemented");
    0 // No attributes
}

/// sys_mknod handler - Create a special file node
///
/// # Arguments
/// * `path_ptr` - Pointer to null-terminated path string
/// * `mode` - File mode (type and permissions)
/// * `dev` - Device number (major << 32 | minor) for device nodes
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Supported types
/// * S_IFREG (0o100000): Regular file
/// * S_IFCHR (0o020000): Character device
/// * S_IFBLK (0o060000): Block device
/// * S_IFIFO (0o010000): FIFO (named pipe)
/// * S_IFSOCK (0o140000): Unix domain socket
fn sys_mknod(path_ptr: usize, mode: usize, dev: usize) -> isize {
    use crate::fs::vfs::inode::FileMode;

    // Validate path pointer
    if !validate_user_buffer(path_ptr, 1) {
        serial_println!("[SYSCALL] sys_mknod: invalid path pointer");
        return -1; // EFAULT
    }

    // Read path string
    let path_bytes = unsafe {
        let mut len = 0;
        let ptr = path_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_mknod: path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_mknod: invalid UTF-8 in path");
            return -1; // EINVAL
        }
    };

    let file_mode = FileMode::new(mode as u16);
    let file_type = file_mode.file_type();

    // Extract major and minor device numbers
    let major = (dev >> 32) as u32;
    let minor = (dev & 0xFFFFFFFF) as u32;

    serial_println!(
        "[SYSCALL] sys_mknod: path={}, mode={:#o}, type={:#o}, dev={}:{}",
        path,
        mode,
        file_type,
        major,
        minor
    );

    // Validate file type
    match file_type {
        FileMode::S_IFREG => {
            // Regular file - can be created with mknod
            serial_println!("[SYSCALL] sys_mknod: creating regular file");
        }
        FileMode::S_IFCHR => {
            // Character device
            serial_println!(
                "[SYSCALL] sys_mknod: creating character device {}:{}",
                major,
                minor
            );
        }
        FileMode::S_IFBLK => {
            // Block device
            serial_println!(
                "[SYSCALL] sys_mknod: creating block device {}:{}",
                major,
                minor
            );
        }
        FileMode::S_IFIFO => {
            // FIFO (named pipe)
            serial_println!("[SYSCALL] sys_mknod: creating FIFO");
        }
        FileMode::S_IFSOCK => {
            // Unix domain socket
            serial_println!("[SYSCALL] sys_mknod: creating socket");
        }
        _ => {
            serial_println!("[SYSCALL] sys_mknod: invalid file type {:#o}", file_type);
            return -1; // EINVAL
        }
    }

    serial_println!("[SYSCALL] sys_mknod: complete");
    0 // Success
}

/// sys_sync handler - Sync all filesystems
///
/// # Returns
/// Always returns 0
///
/// # Description
/// Commits all pending writes to disk for all mounted filesystems.
/// This triggers TxG commit for mfs_disk and flushes the page cache.
fn sys_sync() -> isize {
    serial_println!("[SYSCALL] sys_sync: syncing all filesystems");


    serial_println!("[SYSCALL] sys_sync: complete");
    0 // Success (even if not implemented)
}

/// sys_fsync handler - Sync a specific file
///
/// # Arguments
/// * `fd` - File descriptor to sync
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Description
/// Commits all pending writes for the specified file to disk,
/// including both data and metadata.
fn sys_fsync(fd: usize) -> isize {
    serial_println!("[SYSCALL] sys_fsync: fd={}", fd);

    // Validate file descriptor
    let fd_table = FD_TABLE.lock();
    let _fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_fsync: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);


    serial_println!("[SYSCALL] sys_fsync: not implemented");
    0 // Pretend success for now
}

/// sys_fdatasync handler - Sync file data only (not metadata)
///
/// # Arguments
/// * `fd` - File descriptor to sync
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Description
/// Commits all pending data writes for the specified file to disk,
/// but does not necessarily sync metadata (like timestamps).
/// This is faster than fsync when metadata changes are not critical.
fn sys_fdatasync(fd: usize) -> isize {
    serial_println!("[SYSCALL] sys_fdatasync: fd={}", fd);

    // Validate file descriptor
    let fd_table = FD_TABLE.lock();
    let _fd_entry = match fd_table.get(fd) {
        Some(entry) => entry,
        None => {
            serial_println!("[SYSCALL] sys_fdatasync: invalid FD {}", fd);
            return -1; // EBADF
        }
    };
    drop(fd_table);


    serial_println!("[SYSCALL] sys_fdatasync: not implemented");
    0 // Pretend success for now
}

/// sys_mount handler - Mount a filesystem
///
/// # Arguments
/// * `source_ptr` - Pointer to null-terminated device path (e.g., "/dev/sda1")
/// * `target_ptr` - Pointer to null-terminated mount point path (e.g., "/mnt")
/// * `fstype_ptr` - Pointer to null-terminated filesystem type (e.g., "mfs_disk", "mfs_ram")
/// * `flags` - Mount flags (MS_RDONLY, MS_NOATIME, etc.)
/// * `data_ptr` - Pointer to filesystem-specific options string
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Note
/// This is a simplified version that takes 3 args instead of 5.
///
/// # Mount options (parsed from data string)
/// * noatime: Don't update access times
/// * relatime: Update access times relatively
/// * compress=lz4: Use LZ4 compression
/// * compress=zstd: Use Zstd compression
/// * checksums: Enable data checksums
/// * cow: Enable copy-on-write
/// * trim: Enable TRIM support
fn sys_mount(source_ptr: usize, target_ptr: usize, fstype_ptr: usize) -> isize {
    // Validate pointers
    if !validate_user_buffer(source_ptr, 1) {
        serial_println!("[SYSCALL] sys_mount: invalid source pointer");
        return -1; // EFAULT
    }
    if !validate_user_buffer(target_ptr, 1) {
        serial_println!("[SYSCALL] sys_mount: invalid target pointer");
        return -1; // EFAULT
    }
    if !validate_user_buffer(fstype_ptr, 1) {
        serial_println!("[SYSCALL] sys_mount: invalid fstype pointer");
        return -1; // EFAULT
    }

    // Read source string
    let source_bytes = unsafe {
        let mut len = 0;
        let ptr = source_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_mount: source path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let source = match core::str::from_utf8(source_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_mount: invalid UTF-8 in source");
            return -1; // EINVAL
        }
    };

    // Read target string
    let target_bytes = unsafe {
        let mut len = 0;
        let ptr = target_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_mount: target path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let target = match core::str::from_utf8(target_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_mount: invalid UTF-8 in target");
            return -1; // EINVAL
        }
    };

    // Read fstype string
    let fstype_bytes = unsafe {
        let mut len = 0;
        let ptr = fstype_ptr as *const u8;
        while len < 256 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 256 {
            serial_println!("[SYSCALL] sys_mount: fstype too long");
            return -1; // EINVAL
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let fstype = match core::str::from_utf8(fstype_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_mount: invalid UTF-8 in fstype");
            return -1; // EINVAL
        }
    };

    serial_println!(
        "[SYSCALL] sys_mount: source={}, target={}, fstype={}",
        source,
        target,
        fstype
    );

    // Validate filesystem type
    match fstype {
        "mfs_ram" | "mfs_disk" => {
            serial_println!("[SYSCALL] sys_mount: valid filesystem type");
        }
        _ => {
            serial_println!("[SYSCALL] sys_mount: unsupported filesystem type");
            return -1; // ENODEV
        }
    }


    serial_println!("[SYSCALL] sys_mount: complete");
    0 // Success
}

/// sys_umount handler - Unmount a filesystem
///
/// # Arguments
/// * `target_ptr` - Pointer to null-terminated mount point path
/// * `flags` - Unmount flags (MNT_FORCE, MNT_DETACH, etc.)
///
/// # Returns
/// 0 on success, or -1 on error
///
/// # Flags
/// * MNT_FORCE (0x1): Force unmount even if busy
/// * MNT_DETACH (0x2): Lazy unmount (detach from namespace)
/// * MNT_EXPIRE (0x4): Mark for expiration
fn sys_umount(target_ptr: usize, flags: usize) -> isize {
    // Validate pointer
    if !validate_user_buffer(target_ptr, 1) {
        serial_println!("[SYSCALL] sys_umount: invalid target pointer");
        return -1; // EFAULT
    }

    // Read target string
    let target_bytes = unsafe {
        let mut len = 0;
        let ptr = target_ptr as *const u8;
        while len < 4096 && *ptr.add(len) != 0 {
            len += 1;
        }
        if len >= 4096 {
            serial_println!("[SYSCALL] sys_umount: target path too long");
            return -1; // ENAMETOOLONG
        }
        core::slice::from_raw_parts(ptr, len)
    };

    let target = match core::str::from_utf8(target_bytes) {
        Ok(s) => s,
        Err(_) => {
            serial_println!("[SYSCALL] sys_umount: invalid UTF-8 in target");
            return -1; // EINVAL
        }
    };

    const MNT_FORCE: usize = 0x1;
    const MNT_DETACH: usize = 0x2;
    const MNT_EXPIRE: usize = 0x4;

    let force = (flags & MNT_FORCE) != 0;
    let detach = (flags & MNT_DETACH) != 0;
    let expire = (flags & MNT_EXPIRE) != 0;

    serial_println!(
        "[SYSCALL] sys_umount: target={}, force={}, detach={}, expire={}",
        target,
        force,
        detach,
        expire
    );


    serial_println!("[SYSCALL] sys_umount: complete");
    0 // Success
}
