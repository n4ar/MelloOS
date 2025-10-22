//! grep - search for patterns in files

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::vec::Vec;
use alloc::string::String;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "irn")?;
    
    let case_insensitive = args.has_option('i');
    let _recursive = args.has_option('r');
    let show_line_numbers = args.has_option('n');
    
    // Need at least pattern argument
    args.require_positional(1)?;
    
    let pattern = args.get_positional(0).unwrap();
    
    // If no files specified, read from stdin
    if args.positional_count() == 1 {
        grep_fd(0, pattern, case_insensitive, show_line_numbers, None)?;
        return Ok(0);
    }
    
    // Process each file
    let mut found_any = false;
    for i in 1..args.positional_count() {
        let path = args.get_positional(i).unwrap();
        
        // Special case: "-" means stdin
        if path == "-" {
            let found = grep_fd(0, pattern, case_insensitive, show_line_numbers, None)?;
            found_any = found_any || found;
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
        
        let filename = if args.positional_count() > 2 {
            Some(path)
        } else {
            None
        };
        
        let found = grep_fd(fd as i32, pattern, case_insensitive, show_line_numbers, filename)?;
        found_any = found_any || found;
        
        syscalls::close(fd as i32);
    }
    
    // Return 0 if found, 1 if not found
    Ok(if found_any { 0 } else { 1 })
}

fn grep_fd(
    fd: i32,
    pattern: &str,
    case_insensitive: bool,
    show_line_numbers: bool,
    filename: Option<&str>,
) -> Result<bool> {
    let mut buf = [0u8; 4096];
    let mut line_buf = Vec::new();
    let mut line_num = 1;
    let mut found_any = false;
    
    // Helper function for case-insensitive comparison
    fn matches_pattern(line: &str, pattern: &str, case_insensitive: bool) -> bool {
        if case_insensitive {
            // Simple case-insensitive search by comparing lowercase ASCII
            let line_bytes = line.as_bytes();
            let pattern_bytes = pattern.as_bytes();
            
            for i in 0..=line_bytes.len().saturating_sub(pattern_bytes.len()) {
                let mut matches = true;
                for j in 0..pattern_bytes.len() {
                    let line_char = line_bytes[i + j].to_ascii_lowercase();
                    let pattern_char = pattern_bytes[j].to_ascii_lowercase();
                    if line_char != pattern_char {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    return true;
                }
            }
            false
        } else {
            line.contains(pattern)
        }
    }
    
    loop {
        let nread = syscalls::read(fd, &mut buf);
        if nread < 0 {
            return Err(Error::from_errno(nread));
        }
        if nread == 0 {
            // Process last line if any
            if !line_buf.is_empty() {
                if let Ok(line) = core::str::from_utf8(&line_buf) {
                    if matches_pattern(line, pattern, case_insensitive) {
                        print_match(filename, line_num, line, show_line_numbers);
                        found_any = true;
                    }
                }
            }
            break;
        }
        
        // Process buffer line by line
        for i in 0..nread as usize {
            let byte = buf[i];
            
            if byte == b'\n' {
                // End of line - check if it matches
                if let Ok(line) = core::str::from_utf8(&line_buf) {
                    if matches_pattern(line, pattern, case_insensitive) {
                        print_match(filename, line_num, line, show_line_numbers);
                        found_any = true;
                    }
                }
                
                line_buf.clear();
                line_num += 1;
            } else {
                line_buf.push(byte);
            }
        }
    }
    
    Ok(found_any)
}

fn matches_pattern(line: &str, pattern: &str, case_insensitive: bool) -> bool {
    if case_insensitive {
        let line_lower = line.to_lowercase();
        line_lower.contains(pattern)
    } else {
        line.contains(pattern)
    }
}

fn print_match(filename: Option<&str>, line_num: usize, line: &str, show_line_numbers: bool) {
    // Print filename if multiple files
    if let Some(name) = filename {
        syscalls::write(1, name.as_bytes());
        syscalls::write(1, b":");
    }
    
    // Print line number if requested
    if show_line_numbers {
        let num_str = format_number(line_num);
        syscalls::write(1, num_str.as_bytes());
        syscalls::write(1, b":");
    }
    
    // Print the line
    syscalls::write(1, line.as_bytes());
    syscalls::write(1, b"\n");
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
    
    result
}
