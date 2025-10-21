//! cp - copy files and directories

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "riv")?;
    
    let recursive = args.has_option('r');
    let interactive = args.has_option('i');
    let verbose = args.has_option('v');
    
    // Need at least 2 arguments (source and destination)
    args.require_positional(2)?;
    
    let source = args.get_positional(0).unwrap();
    let dest = args.get_positional(1).unwrap();
    
    // Copy the file
    copy_file(source, dest, recursive, interactive, verbose)?;
    
    Ok(0)
}

fn copy_file(source: &str, dest: &str, _recursive: bool, interactive: bool, verbose: bool) -> Result<()> {
    // Check if destination exists
    let mut dest_bytes = Vec::new();
    dest_bytes.extend_from_slice(dest.as_bytes());
    dest_bytes.push(0);
    
    let dest_fd = syscalls::openat(
        syscalls::AT_FDCWD,
        &dest_bytes,
        syscalls::O_RDONLY,
        0,
    );
    
    let dest_exists = dest_fd >= 0;
    if dest_exists {
        syscalls::close(dest_fd as i32);
        
        // If interactive, prompt user
        if interactive {
            syscalls::write(1, b"overwrite '");
            syscalls::write(1, dest.as_bytes());
            syscalls::write(1, b"'? (y/n) ");
            
            let mut response = [0u8; 1];
            let nread = syscalls::read(0, &mut response);
            if nread <= 0 || (response[0] != b'y' && response[0] != b'Y') {
                return Ok(());
            }
        }
    }
    
    // Open source file
    let mut source_bytes = Vec::new();
    source_bytes.extend_from_slice(source.as_bytes());
    source_bytes.push(0);
    
    let src_fd = syscalls::openat(
        syscalls::AT_FDCWD,
        &source_bytes,
        syscalls::O_RDONLY,
        0,
    );
    
    if src_fd < 0 {
        return Err(Error::from_errno(src_fd));
    }
    
    // Create/open destination file
    let dst_fd = syscalls::openat(
        syscalls::AT_FDCWD,
        &dest_bytes,
        syscalls::O_WRONLY | syscalls::O_CREAT | syscalls::O_TRUNC,
        syscalls::S_IRUSR | syscalls::S_IWUSR | syscalls::S_IRWXU,
    );
    
    if dst_fd < 0 {
        syscalls::close(src_fd as i32);
        return Err(Error::from_errno(dst_fd));
    }
    
    // Copy data in chunks
    let mut buf = [0u8; 4096];
    loop {
        let nread = syscalls::read(src_fd as i32, &mut buf);
        if nread < 0 {
            syscalls::close(src_fd as i32);
            syscalls::close(dst_fd as i32);
            return Err(Error::from_errno(nread));
        }
        if nread == 0 {
            break;
        }
        
        let nwritten = syscalls::write(dst_fd as i32, &buf[..nread as usize]);
        if nwritten < 0 {
            syscalls::close(src_fd as i32);
            syscalls::close(dst_fd as i32);
            return Err(Error::from_errno(nwritten));
        }
        if nwritten != nread {
            syscalls::close(src_fd as i32);
            syscalls::close(dst_fd as i32);
            return Err(Error::IoError);
        }
    }
    
    syscalls::close(src_fd as i32);
    syscalls::close(dst_fd as i32);
    
    if verbose {
        syscalls::write(1, b"'");
        syscalls::write(1, source.as_bytes());
        syscalls::write(1, b"' -> '");
        syscalls::write(1, dest.as_bytes());
        syscalls::write(1, b"'\n");
    }
    
    Ok(())
}
