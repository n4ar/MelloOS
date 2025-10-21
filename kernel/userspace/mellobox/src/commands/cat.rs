//! cat - concatenate files and print on standard output

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;
use alloc::string::String;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "n")?;
    
    let number_lines = args.has_option('n');
    
    // If no files specified, read from stdin
    if args.positional_count() == 0 {
        cat_fd(0, number_lines, 1)?;
        return Ok(0);
    }
    
    // Process each file
    for i in 0..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        
        // Special case: "-" means stdin
        if path == "-" {
            cat_fd(0, number_lines, 1)?;
            continue;
        }
        
        // Open file
        let mut path_bytes = Vec::new();
        path_bytes.extend_from_slice(path.as_bytes());
        path_bytes.push(0);
        
        let fd = syscalls::openat(
            syscalls::AT_FDCWD,
            &path_bytes,
            syscalls::O_RDONLY,
            0,
        );
        
        if fd < 0 {
            return Err(Error::from_errno(fd));
        }
        
        let result = cat_fd(fd as i32, number_lines, 1);
        syscalls::close(fd as i32);
        
        result?;
    }
    
    Ok(0)
}

fn cat_fd(fd: i32, number_lines: bool, line_num_start: usize) -> Result<()> {
    let mut buf = [0u8; 4096];
    let mut line_num = line_num_start;
    let mut at_line_start = true;
    
    loop {
        let nread = syscalls::read(fd, &mut buf);
        if nread < 0 {
            return Err(Error::from_errno(nread));
        }
        if nread == 0 {
            break;
        }
        
        if number_lines {
            // Process byte by byte to add line numbers
            for i in 0..nread as usize {
                if at_line_start {
                    // Print line number
                    let line_str = format_number(line_num);
                    syscalls::write(1, line_str.as_bytes());
                    syscalls::write(1, b"  ");
                    line_num += 1;
                    at_line_start = false;
                }
                
                syscalls::write(1, &buf[i..i+1]);
                
                if buf[i] == b'\n' {
                    at_line_start = true;
                }
            }
        } else {
            // Just write the data
            let nwritten = syscalls::write(1, &buf[..nread as usize]);
            if nwritten < 0 {
                return Err(Error::from_errno(nwritten));
            }
        }
    }
    
    Ok(())
}

fn format_number(n: usize) -> String {
    if n == 0 {
        return String::from("0");
    }
    
    let mut result = String::new();
    let mut num = n;
    
    while num > 0 {
        let digit = (num % 10) as u8;
        result.insert(0, (b'0' + digit) as char);
        num /= 10;
    }
    
    // Pad to 6 characters
    while result.len() < 6 {
        result.insert(0, ' ');
    }
    
    result
}
