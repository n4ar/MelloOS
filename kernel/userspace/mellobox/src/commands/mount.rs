//! mount - mount a filesystem

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::format;
use alloc::vec::Vec;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "t:o:")?;

    // If no arguments, show mounted filesystems
    if args.positional_count() == 0 {
        syscalls::write(1, b"mfs_ram on / type mfs_ram (rw)\n");
        return Ok(0);
    }

    // Need at least source and target
    if args.positional_count() < 2 {
        syscalls::write(2, b"mount: missing operand\n");
        syscalls::write(
            2,
            b"Usage: mount [-t type] [-o options] <source> <target>\n",
        );
        return Ok(1);
    }

    let source = args.get_positional(0).unwrap();
    let target = args.get_positional(1).unwrap();

    // Get filesystem type (default to auto-detect)
    let fstype = args.get_option('t').unwrap_or("auto");

    // Get mount options (default to empty)
    let options = args.get_option('o').unwrap_or("");

    // Prepare null-terminated strings
    let mut source_bytes = Vec::new();
    source_bytes.extend_from_slice(source.as_bytes());
    source_bytes.push(0);

    let mut target_bytes = Vec::new();
    target_bytes.extend_from_slice(target.as_bytes());
    target_bytes.push(0);

    let mut fstype_bytes = Vec::new();
    fstype_bytes.extend_from_slice(fstype.as_bytes());
    fstype_bytes.push(0);

    let mut options_bytes = Vec::new();
    options_bytes.extend_from_slice(options.as_bytes());
    options_bytes.push(0);

    // Call mount syscall
    let result = syscalls::mount(
        source_bytes.as_ptr(),
        target_bytes.as_ptr(),
        fstype_bytes.as_ptr(),
        0, // flags
        options_bytes.as_ptr(),
    );

    if result < 0 {
        let msg = format!("mount: failed to mount {} on {}\n", source, target);
        syscalls::write(2, msg.as_bytes());
        return Ok(1);
    }

    let msg = format!("Mounted {} on {} (type {})\n", source, target, fstype);
    syscalls::write(1, msg.as_bytes());

    Ok(0)
}
