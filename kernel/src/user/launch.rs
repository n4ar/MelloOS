//! User-mode launcher helper
//!
//! Provides a small wrapper around the assembly trampoline so we can
//! validate and log the entry parameters before jumping to ring 3.

use crate::arch::x86_64::user_entry_trampoline;
use crate::sched::task::USER_LIMIT;
use crate::serial_println;

/// Launch a userspace entry point with the provided stack.
///
/// # Safety
/// The caller must ensure `entry` and `stack_top` are canonical user addresses
/// and that the corresponding pages have been mapped with the correct
/// permissions (user | present, and stack writable).
pub fn launch(entry: u64, stack_top: u64) -> ! {
    serial_println!(
        "[USER-LAUNCH] entry=0x{:x} stack_top=0x{:x}",
        entry,
        stack_top
    );

    assert!(
        entry < USER_LIMIT as u64,
        "User entry point 0x{:x} outside user space",
        entry
    );
    assert!(
        stack_top < USER_LIMIT as u64,
        "User stack top 0x{:x} outside user space",
        stack_top
    );
    assert!(
        stack_top & 0xF == 0,
        "User stack top 0x{:x} not 16-byte aligned",
        stack_top
    );

    #[cfg(debug_assertions)]
    {
        let mut cs: u16;
        let mut ss: u16;
        let mut rsp: u64;
        let mut rflags: u64;
        unsafe {
            core::arch::asm!("mov {0:x}, cs", out(reg) cs);
            core::arch::asm!("mov {0:x}, ss", out(reg) ss);
            core::arch::asm!("mov {0}, rsp", out(reg) rsp);
            core::arch::asm!("pushfq; pop {0}", out(reg) rflags);
        }
        serial_println!(
            "[USER-LAUNCH] pre-trampoline state: cs=0x{:x} ss=0x{:x} rsp=0x{:x} rflags=0x{:x}",
            cs,
            ss,
            rsp,
            rflags
        );
        serial_println!(
            "[USER-LAUNCH] Proceeding to user-mode trampoline (entry=0x{:x}, stack=0x{:x})",
            entry,
            stack_top
        );
    }

    unsafe { user_entry_trampoline(entry, stack_top) }
}
