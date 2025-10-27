//! umount - unmount a filesystem

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::format;
use alloc::vec::Vec;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "fl")?;

    if args.positional_count() == 0 {
        syscalls::write(2, b"umount: missing operand\n");
        syscalls::write(2, b"Usage: umount [-f] [-l] <target>\n");
        return Ok(1);
    }

    let target = args.get_positional(0).unwrap();

    // Parse flags
    let force = args.has_option('f');
    let lazy = args.has_option('l');

    let mut flags = 0usize;
    if force {
        flags |= 0x1; // MNT_FORCE
    }
    if lazy {
        flags |= 0x2; // MNT_DETACH
    }

    // Prepare null-terminated string
    let mut target_bytes = Vec::new();
    target_bytes.extend_from_slice(target.as_bytes());
    target_bytes.push(0);

    // Call umount syscall
    let result = syscalls::umount(target_bytes.as_ptr(), flags);

    if result < 0 {
        let msg = format!("umount: failed to unmount {}\n", target);
        syscalls::write(2, msg.as_bytes());
        return Ok(1);
    }

    let msg = format!("Unmounted {}\n", target);
    syscalls::write(1, msg.as_bytes());

    Ok(0)
}
