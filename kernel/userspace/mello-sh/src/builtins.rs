//! Built-in commands for mello-sh

use alloc::string::String;
use alloc::format;
use crate::{Shell, syscalls, jobs::JobState};

/// Execute a built-in command
/// Returns Some(status) if command was a built-in, None otherwise
pub fn execute(shell: &mut Shell, cmd: &str, args: &[String]) -> Option<i32> {
    match cmd {
        "cd" => Some(builtin_cd(shell, args)),
        "pwd" => Some(builtin_pwd()),
        "echo" => Some(builtin_echo(args)),
        "export" => Some(builtin_export(shell, args)),
        "unset" => Some(builtin_unset(shell, args)),
        "jobs" => Some(builtin_jobs(shell)),
        "fg" => Some(builtin_fg(shell, args)),
        "bg" => Some(builtin_bg(shell, args)),
        "exit" => Some(builtin_exit(shell, args)),
        "which" => Some(builtin_which(shell, args)),
        "debug-pty" => Some(builtin_debug_pty()),
        "debug-jobs" => Some(builtin_debug_jobs(shell)),
        "debug-signals" => Some(builtin_debug_signals()),
        _ => None,
    }
}

/// cd - change directory
fn builtin_cd(shell: &mut Shell, args: &[String]) -> i32 {
    // Determine target directory
    let target = if args.is_empty() {
        // Default to $HOME
        if let Some(home) = shell.get_env("HOME") {
            home.clone()
        } else {
            syscalls::write(2, b"cd: HOME not set\n");
            return 1;
        }
    } else {
        args[0].clone()
    };

    // Add null terminator for syscall
    let mut path_with_null = target.clone();
    path_with_null.push('\0');

    // Call chdir system call
    if syscalls::chdir(path_with_null.as_bytes()) < 0 {
        syscalls::write(2, b"cd: ");
        syscalls::write(2, target.as_bytes());
        syscalls::write(2, b": No such file or directory\n");
        return 1;
    }

    // Update PWD environment variable
    let mut buf = [0u8; 4096];
    let len = syscalls::getcwd(&mut buf);
    
    if len > 0 {
        if let Ok(pwd) = core::str::from_utf8(&buf[..len as usize]) {
            shell.set_env(String::from("PWD"), String::from(pwd));
        }
    }

    0
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

/// export - set environment variable
fn builtin_export(shell: &mut Shell, args: &[String]) -> i32 {
    if args.is_empty() {
        // Print all environment variables
        for (key, value) in shell.env() {
            syscalls::write(1, b"export ");
            syscalls::write(1, key.as_bytes());
            syscalls::write(1, b"=");
            syscalls::write(1, value.as_bytes());
            syscalls::write(1, b"\n");
        }
        return 0;
    }

    // Parse VAR=value syntax
    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            
            if key.is_empty() {
                syscalls::write(2, b"export: invalid variable name\n");
                return 1;
            }

            shell.set_env(String::from(key), String::from(value));
        } else {
            // Just mark variable for export (already in environment)
            // For now, we don't distinguish between exported and non-exported
            if shell.get_env(arg).is_none() {
                syscalls::write(2, b"export: ");
                syscalls::write(2, arg.as_bytes());
                syscalls::write(2, b": not found\n");
                return 1;
            }
        }
    }

    0
}

/// unset - unset environment variable
fn builtin_unset(shell: &mut Shell, args: &[String]) -> i32 {
    if args.is_empty() {
        syscalls::write(2, b"unset: missing argument\n");
        return 1;
    }

    for var in args {
        shell.unset_env(var);
    }

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
fn builtin_which(shell: &Shell, args: &[String]) -> i32 {
    if args.is_empty() {
        syscalls::write(2, b"which: missing argument\n");
        return 1;
    }

    let mut found_all = true;

    for cmd in args {
        // Check if it's a built-in
        if matches!(cmd.as_str(), "cd" | "pwd" | "echo" | "export" | "unset" | "jobs" | "fg" | "bg" | "exit" | "which" | "debug-pty" | "debug-jobs" | "debug-signals") {
            syscalls::write(1, format!("{}: shell built-in command\n", cmd).as_bytes());
            continue;
        }

        // Search PATH environment variable
        let path_var = shell.get_env("PATH").map(|s| s.as_str()).unwrap_or("/bin");
        let mut found = false;

        // Split PATH by colon
        for dir in path_var.split(':') {
            if dir.is_empty() {
                continue;
            }

            // Construct full path
            let mut full_path = String::from(dir);
            if !full_path.ends_with('/') {
                full_path.push('/');
            }
            full_path.push_str(cmd);
            full_path.push('\0');

            // Try to open the file to check if it exists
            let fd = syscalls::open(full_path.as_bytes(), syscalls::O_RDONLY, 0);
            if fd >= 0 {
                syscalls::close(fd as i32);
                // Print without null terminator
                let output = &full_path[..full_path.len() - 1];
                syscalls::write(1, output.as_bytes());
                syscalls::write(1, b"\n");
                found = true;
                break;
            }
        }

        if !found {
            syscalls::write(2, format!("which: {}: not found\n", cmd).as_bytes());
            found_all = false;
        }
    }

    if found_all { 0 } else { 1 }
}

/// debug-pty - show PTY state from /proc/debug/pty
fn builtin_debug_pty() -> i32 {
    syscalls::write(1, b"=== PTY Debug Information ===\n");
    
    // Read /proc/debug/pty
    let path = b"/proc/debug/pty\0";
    let fd = syscalls::open(path, syscalls::O_RDONLY, 0);
    
    if fd < 0 {
        syscalls::write(2, b"debug-pty: failed to open /proc/debug/pty\n");
        return 1;
    }
    
    // Read and display content
    let mut buf = [0u8; 4096];
    loop {
        let n = syscalls::read(fd as i32, &mut buf);
        if n <= 0 {
            break;
        }
        syscalls::write(1, &buf[..n as usize]);
    }
    
    syscalls::close(fd as i32);
    0
}

/// debug-jobs - show detailed job table
fn builtin_debug_jobs(shell: &Shell) -> i32 {
    syscalls::write(1, b"=== Job Table Debug Information ===\n");
    
    // Get all jobs from the job table
    let all_jobs = shell.get_all_jobs();
    
    if all_jobs.is_empty() {
        syscalls::write(1, b"No jobs\n");
        return 0;
    }
    
    for job in all_jobs {
        // Print job header
        let header = format!(
            "[{}] PGID={} State={:?} Background={}\n",
            job.id, job.pgid, job.state, job.background
        );
        syscalls::write(1, header.as_bytes());
        
        // Print command
        syscalls::write(1, b"  Command: ");
        syscalls::write(1, job.command.as_bytes());
        syscalls::write(1, b"\n");
        
        // Print process list
        syscalls::write(1, b"  Processes: [");
        for (i, pid) in job.processes.iter().enumerate() {
            if i > 0 {
                syscalls::write(1, b", ");
            }
            let pid_str = format!("{}", pid);
            syscalls::write(1, pid_str.as_bytes());
        }
        syscalls::write(1, b"]\n");
        
        // Read process state from /proc if available
        for pid in &job.processes {
            let proc_path = format!("/proc/{}/stat\0", pid);
            let fd = syscalls::open(proc_path.as_bytes(), syscalls::O_RDONLY, 0);
            
            if fd >= 0 {
                let mut buf = [0u8; 512];
                let n = syscalls::read(fd as i32, &mut buf);
                if n > 0 {
                    syscalls::write(1, b"    PID ");
                    let pid_str = format!("{}", pid);
                    syscalls::write(1, pid_str.as_bytes());
                    syscalls::write(1, b": ");
                    syscalls::write(1, &buf[..n as usize]);
                }
                syscalls::close(fd as i32);
            }
        }
        
        syscalls::write(1, b"\n");
    }
    
    0
}

/// debug-signals - show pending signals for current process
fn builtin_debug_signals() -> i32 {
    syscalls::write(1, b"=== Signal Debug Information ===\n");
    
    // Get current PID
    let pid = syscalls::getpid();
    let pid_str = format!("Current PID: {}\n", pid);
    syscalls::write(1, pid_str.as_bytes());
    
    // Read /proc/<pid>/status to get signal information
    let status_path = format!("/proc/{}/status\0", pid);
    let fd = syscalls::open(status_path.as_bytes(), syscalls::O_RDONLY, 0);
    
    if fd < 0 {
        syscalls::write(2, b"debug-signals: failed to open /proc status\n");
        return 1;
    }
    
    // Read and display content
    let mut buf = [0u8; 2048];
    let n = syscalls::read(fd as i32, &mut buf);
    if n > 0 {
        syscalls::write(1, &buf[..n as usize]);
    }
    
    syscalls::close(fd as i32);
    
    // Also read /proc/debug/sessions for session info
    syscalls::write(1, b"\n=== Session Information ===\n");
    let sessions_path = b"/proc/debug/sessions\0";
    let fd = syscalls::open(sessions_path, syscalls::O_RDONLY, 0);
    
    if fd >= 0 {
        let mut buf = [0u8; 4096];
        let n = syscalls::read(fd as i32, &mut buf);
        if n > 0 {
            syscalls::write(1, &buf[..n as usize]);
        }
        syscalls::close(fd as i32);
    }
    
    0
}
