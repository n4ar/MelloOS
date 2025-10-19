#![no_std]
#![no_main]

use core::arch::asm;

// Syscall numbers
const SYS_WRITE: usize = 0;
const SYS_EXIT: usize = 1;
const SYS_SLEEP: usize = 2;
const SYS_IPC_SEND: usize = 3;
const SYS_IPC_RECV: usize = 4;

/// Raw syscall function using int 0x80
#[inline(always)]
unsafe fn syscall(id: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") id => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        options(nostack)
    );
    ret
}

/// Write data to stdout (fd=0)
fn sys_write(msg: &str) -> isize {
    unsafe {
        syscall(SYS_WRITE, 0, msg.as_ptr() as usize, msg.len())
    }
}

/// Sleep for specified number of ticks
fn sys_sleep(ticks: usize) -> isize {
    unsafe {
        syscall(SYS_SLEEP, ticks, 0, 0)
    }
}

/// Send message to IPC port
fn sys_ipc_send(port_id: usize, data: &[u8]) -> isize {
    unsafe {
        syscall(SYS_IPC_SEND, port_id, data.as_ptr() as usize, data.len())
    }
}

/// Receive message from IPC port (blocking)
fn sys_ipc_recv(port_id: usize, buf: &mut [u8]) -> isize {
    unsafe {
        syscall(SYS_IPC_RECV, port_id, buf.as_mut_ptr() as usize, buf.len())
    }
}

/// Exit current task
fn sys_exit(code: usize) -> ! {
    unsafe {
        syscall(SYS_EXIT, code, 0, 0);
    }
    loop {}
}

/// Entry point for init process
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Print hello message
    sys_write("Hello from userland! ✨\n");
    
    // Demonstrate IPC by sending "ping" to port 2
    let ping_msg = b"ping";
    let send_result = sys_ipc_send(2, ping_msg);
    if send_result >= 0 {
        sys_write("Sent 'ping' to port 2\n");
    } else {
        sys_write("Failed to send to port 2\n");
    }
    
    // Demonstrate IPC by receiving from port 1
    let mut recv_buf = [0u8; 64];
    sys_write("Waiting to receive from port 1...\n");
    let recv_result = sys_ipc_recv(1, &mut recv_buf);
    
    if recv_result > 0 {
        sys_write("Received message from port 1: ");
        // Print the received message (up to recv_result bytes)
        let msg_len = recv_result as usize;
        if msg_len <= recv_buf.len() {
            let received = core::str::from_utf8(&recv_buf[..msg_len])
                .unwrap_or("<invalid utf8>");
            sys_write(received);
            sys_write("\n");
        }
    } else {
        sys_write("Failed to receive from port 1\n");
    }
    
    // Sleep for 100 ticks
    sys_write("Sleeping for 100 ticks...\n");
    sys_sleep(100);
    
    // Print wake up message
    sys_write("Woke up!\n");
    
    // Enter infinite loop with periodic sleep
    let mut counter = 0u32;
    loop {
        sys_write("Init process running...\n");
        sys_sleep(1000);
        counter = counter.wrapping_add(1);
    }
}

// Panic handler for userspace
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
