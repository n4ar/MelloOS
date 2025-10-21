//! touch - create empty file or update timestamp

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "")?;
    
    // Need at least one file argument
    args.require_positional(1)?;
    
    // Touch each file
    for i in 0..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        touch_file(path)?;
    }
    
    Ok(0)
}

fn touch_file(path: &str) -> Result<()> {
    let mut path_bytes = Vec::new();
    path_bytes.extend_from_slice(path.as_bytes());
    path_bytes.push(0);
    
    // Try to open the file (create if doesn't exist)
    let fd = syscalls::openat(
        syscalls::AT_FDCWD,
        &path_bytes,
        syscalls::O_WRONLY | syscalls::O_CREAT,
        syscalls::S_IRUSR | syscalls::S_IWUSR,
    );
    
    if fd < 0 {
        return Err(Error::from_errno(fd));
    }
    
    // Close the file
    syscalls::close(fd as i32);
    
    Ok(())
}
