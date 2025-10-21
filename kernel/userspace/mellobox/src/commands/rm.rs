//! rm - remove files or directories

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "rfi")?;
    
    let recursive = args.has_option('r');
    let force = args.has_option('f');
    let interactive = args.has_option('i');
    
    // Need at least 1 file argument
    if args.positional_count() == 0 {
        if !force {
            return Err(Error::MissingArgument);
        }
        return Ok(0);
    }
    
    // Remove each file
    for i in 0..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        
        // If interactive, prompt user
        if interactive {
            syscalls::write(1, b"remove '");
            syscalls::write(1, path.as_bytes());
            syscalls::write(1, b"'? (y/n) ");
            
            let mut response = [0u8; 1];
            let nread = syscalls::read(0, &mut response);
            if nread <= 0 || (response[0] != b'y' && response[0] != b'Y') {
                continue;
            }
        }
        
        let result = remove_file(path, recursive);
        
        if result.is_err() && !force {
            return result.map(|_| 1);
        }
    }
    
    Ok(0)
}

fn remove_file(path: &str, recursive: bool) -> Result<()> {
    let mut path_bytes = Vec::new();
    path_bytes.extend_from_slice(path.as_bytes());
    path_bytes.push(0);
    
    // Try to remove as file first
    let result = syscalls::unlink(&path_bytes);
    
    if result == 0 {
        return Ok(());
    }
    
    // If unlink failed, might be a directory
    let errno = -result;
    
    // EISDIR (21) means it's a directory
    if errno == 21 {
        if !recursive {
            return Err(Error::IsADirectory);
        }
        
        // Try to remove as directory
        let rmdir_result = syscalls::rmdir(&path_bytes);
        if rmdir_result == 0 {
            return Ok(());
        }
        
        return Err(Error::from_errno(rmdir_result));
    }
    
    Err(Error::from_errno(result))
}
