//! pwd - print working directory

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let _args = Args::parse(argv, "")?;

    // Get current working directory
    let mut buf = [0u8; 4096];
    let result = syscalls::getcwd(&mut buf);

    if result < 0 {
        return Err(Error::from_errno(result));
    }

    // Find the length (null-terminated)
    let len = buf.iter().position(|&b| b == 0).unwrap_or(result as usize);

    // Print the path
    syscalls::write(1, &buf[..len]);
    syscalls::write(1, b"\n");

    Ok(0)
}
