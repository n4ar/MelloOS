//! Built-in commands for mello-sh

use alloc::string::String;
use alloc::format;
use crate::{Shell, syscalls, jobs::JobState};

/// Execute a built-in command
/// Returns Some(status) if command was a built-in, None otherwise
pub fn execute(shell: &mut Shell, cmd: &str, args: &[String]) -> Option<i32> {
    match cmd {
        "cd" => Some(builtin_cd(args)),
        "pwd" => Some(builtin_pwd()),
        "echo" => Some(builtin_echo(args)),
        "export" => Some(builtin_export(args)),
        "unset" => Some(builtin_unset(args)),
        "jobs" => Some(builtin_jobs(shell)),
        "fg" => Some(builtin_fg(shell, args)),
        "bg" => Some(builtin_bg(shell, args)),
        "exit" => Some(builtin_exit(shell, args)),
        "which" => Some(builtin_which(args)),
        _ => None,
    }
}

/// cd - change directory
fn builtin_cd(args: &[String]) -> i32 {
    let path = if args.is_empty() {
        "/\0" // Default to root for now (should be $HOME)
    } else {
        let mut p = args[0].clone();
        p.push('\0');
        return if syscalls::chdir(p.as_bytes()) < 0 {
            syscalls::write(2, b"cd: failed\n");
            1
        } else {
            0
        };
    };

    if syscalls::chdir(path.as_bytes()) < 0 {
        syscalls::write(2, b"cd: failed\n");
        1
    } else {
        0
    }
}

/// pwd - print working directory
fn builtin_pwd() -> i32 {
    let mut buf = [0u8; 4096];
    let len = syscalls::getcwd(&mut buf);
    
    if len < 0 {
        syscalls::write(2, b"pwd: failed\n");
        return 1;
    }

    syscalls::write(1, &buf[..len as usize]);
    syscalls::write(1, b"\n");
    0
}

/// echo - print arguments
fn builtin_echo(args: &[String]) -> i32 {
    let mut newline = true;
    let mut interpret_escapes = false;
    let mut start = 0;

    // Parse flags
    for (i, arg) in args.iter().enumerate() {
        if arg == "-n" {
            newline = false;
            start = i + 1;
        } else if arg == "-e" {
            interpret_escapes = true;
            start = i + 1;
        } else {
            break;
        }
    }

    // Print arguments
    for (i, arg) in args[start..].iter().enumerate() {
        if i > 0 {
            syscalls::write(1, b" ");
        }
        
        if interpret_escapes {
            // Simple escape handling
            let mut output = String::new();
            let mut chars = arg.chars();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    if let Some(next) = chars.next() {
                        match next {
                            'n' => output.push('\n'),
                            't' => output.push('\t'),
                            '\\' => output.push('\\'),
                            _ => {
                                output.push('\\');
                                output.push(next);
                            }
                        }
                    }
                } else {
                    output.push(ch);
                }
            }
            syscalls::write(1, output.as_bytes());
        } else {
            syscalls::write(1, arg.as_bytes());
        }
    }

    if newline {
        syscalls::write(1, b"\n");
    }

    0
}

/// export - set environment variable (stub)
fn builtin_export(_args: &[String]) -> i32 {
    syscalls::write(2, b"export: not implemented\n");
    0
}

/// unset - unset environment variable (stub)
fn builtin_unset(_args: &[String]) -> i32 {
    syscalls::write(2, b"unset: not implemented\n");
    0
}

/// jobs - list background jobs
fn builtin_jobs(shell: &mut Shell) -> i32 {
    for job in shell.jobs_mut().jobs() {
        let state_str = match job.state {
            JobState::Running => "Running",
            JobState::Stopped => "Stopped",
            JobState::Done(status) => {
                syscalls::write(1, format!("[{}]+ Done({})    {}\n", job.id, status, job.command).as_bytes());
                continue;
            }
        };

        syscalls::write(1, format!("[{}]+ {}    {}\n", job.id, state_str, job.command).as_bytes());
    }
    0
}

/// fg - bring job to foreground
fn builtin_fg(shell: &mut Shell, args: &[String]) -> i32 {
    // Get job ID
    let job_id = if args.is_empty() {
        if let Some(id) = shell.jobs_mut().current_job_id() {
            id
        } else {
            syscalls::write(2, b"fg: no current job\n");
            return 1;
        }
    } else {
        // Parse job ID (format: %N or just N)
        let id_str = args[0].trim_start_matches('%');
        match id_str.parse::<usize>() {
            Ok(id) => id,
            Err(_) => {
                syscalls::write(2, b"fg: invalid job id\n");
                return 1;
            }
        }
    };

    // Find job
    let job = if let Some(j) = shell.jobs_mut().get_job(job_id) {
        j.clone()
    } else {
        syscalls::write(2, b"fg: job not found\n");
        return 1;
    };

    // Set as foreground group
    if let Some(tty_fd) = shell.tty_fd {
        syscalls::tcsetpgrp(tty_fd, job.pgid);
    }

    // Send SIGCONT if stopped
    if job.state == JobState::Stopped {
        syscalls::kill(job.pgid, syscalls::SIGCONT);
    }

    // Wait for job
    let mut status = 0;
    syscalls::wait4(job.pgid, &mut status, syscalls::WUNTRACED);

    // Restore shell as foreground
    if let Some(tty_fd) = shell.tty_fd {
        let shell_pgid = syscalls::getpgrp() as i32;
        syscalls::tcsetpgrp(tty_fd, shell_pgid);
    }

    // Update job state
    if let Some(job) = shell.jobs_mut().get_job_mut(job_id) {
        if status & 0x7f == 0 {
            let exit_status = (status >> 8) & 0xff;
            job.state = JobState::Done(exit_status);
        } else if status & 0xff == 0x7f {
            job.state = JobState::Stopped;
        }
    }

    0
}

/// bg - resume job in background
fn builtin_bg(shell: &mut Shell, args: &[String]) -> i32 {
    // Get job ID
    let job_id = if args.is_empty() {
        if let Some(id) = shell.jobs_mut().current_job_id() {
            id
        } else {
            syscalls::write(2, b"bg: no current job\n");
            return 1;
        }
    } else {
        let id_str = args[0].trim_start_matches('%');
        match id_str.parse::<usize>() {
            Ok(id) => id,
            Err(_) => {
                syscalls::write(2, b"bg: invalid job id\n");
                return 1;
            }
        }
    };

    // Find job
    let job = if let Some(j) = shell.jobs_mut().get_job(job_id) {
        j.clone()
    } else {
        syscalls::write(2, b"bg: job not found\n");
        return 1;
    };

    // Send SIGCONT
    syscalls::kill(job.pgid, syscalls::SIGCONT);

    // Update job state
    if let Some(job) = shell.jobs_mut().get_job_mut(job_id) {
        job.state = JobState::Running;
        syscalls::write(1, format!("[{}]+ {}    {}\n", job.id, "Running", job.command).as_bytes());
    }

    0
}

/// exit - exit shell
fn builtin_exit(shell: &mut Shell, args: &[String]) -> i32 {
    let code = if args.is_empty() {
        0
    } else {
        args[0].parse::<i32>().unwrap_or(0)
    };

    shell.request_exit();
    code
}

/// which - show command path
fn builtin_which(args: &[String]) -> i32 {
    if args.is_empty() {
        syscalls::write(2, b"which: missing argument\n");
        return 1;
    }

    for cmd in args {
        // Check if it's a built-in
        if matches!(cmd.as_str(), "cd" | "pwd" | "echo" | "export" | "unset" | "jobs" | "fg" | "bg" | "exit" | "which") {
            syscalls::write(1, format!("{}: shell built-in command\n", cmd).as_bytes());
        } else {
            // Assume it's in /bin
            syscalls::write(1, format!("/bin/{}\n", cmd).as_bytes());
        }
    }

    0
}
