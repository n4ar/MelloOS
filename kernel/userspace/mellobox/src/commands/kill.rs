//! kill - send a signal to a process

use crate::error::{Error, Result};
use crate::syscalls;

// Signal numbers
const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGQUIT: i32 = 3;
const SIGKILL: i32 = 9;
const SIGTERM: i32 = 15;
const SIGCONT: i32 = 18;
const SIGSTOP: i32 = 19;
const SIGTSTP: i32 = 20;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    // Parse arguments manually since signal can be -<name> or -<number>
    if argv.len() < 2 {
        return Err(Error::MissingArgument);
    }
    
    let mut signal = SIGTERM; // Default signal
    let mut pid_start = 1;
    
    // Check if first argument is a signal
    if argv[1].starts_with('-') {
        let sig_str = &argv[1][1..]; // Skip the '-'
        
        // Try to parse as number
        if let Ok(num) = parse_number(sig_str) {
            signal = num as i32;
        } else {
            // Try to parse as signal name
            signal = match sig_str {
                "HUP" => SIGHUP,
                "INT" => SIGINT,
                "QUIT" => SIGQUIT,
                "KILL" => SIGKILL,
                "TERM" => SIGTERM,
                "CONT" => SIGCONT,
                "STOP" => SIGSTOP,
                "TSTP" => SIGTSTP,
                _ => return Err(Error::InvalidArgument),
            };
        }
        
        pid_start = 2;
    }
    
    // Need at least one PID
    if argv.len() <= pid_start {
        return Err(Error::MissingArgument);
    }
    
    // Send signal to each PID
    for i in pid_start..argv.len() {
        let pid_str = argv[i];
        let pid = parse_number(pid_str).map_err(|_| Error::InvalidArgument)?;
        
        let result = syscalls::kill(pid as i32, signal);
        if result < 0 {
            return Err(Error::from_errno(result));
        }
    }
    
    Ok(0)
}

fn parse_number(s: &str) -> core::result::Result<usize, ()> {
    let mut result = 0;
    
    for c in s.chars() {
        if !c.is_ascii_digit() {
            return Err(());
        }
        result = result * 10 + (c as usize - '0' as usize);
    }
    
    Ok(result)
}
