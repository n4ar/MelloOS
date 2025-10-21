//! Command executor for mello-sh

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::{Shell, parser::{Command, Redirect, RedirectKind}, syscalls, builtins};

/// Execute a command
pub fn execute(shell: &mut Shell, command: Command) -> Result<i32, String> {
    match command {
        Command::Simple { args, background, redirects } => {
            execute_simple(shell, args, background, redirects)
        }
        Command::Pipeline { commands, background } => {
            execute_pipeline(shell, commands, background)
        }
    }
}

/// Execute a simple command
fn execute_simple(
    shell: &mut Shell,
    args: Vec<String>,
    background: bool,
    redirects: Vec<Redirect>,
) -> Result<i32, String> {
    if args.is_empty() {
        return Err("empty command".into());
    }

    let cmd = &args[0];

    // Check for built-in commands
    if let Some(status) = builtins::execute(shell, cmd, &args[1..]) {
        return Ok(status);
    }

    // External command - fork and exec
    let pid = syscalls::fork();
    
    if pid < 0 {
        return Err("fork failed".into());
    }

    if pid == 0 {
        // Child process
        
        // Set process group
        syscalls::setpgid(0, 0);

        // Apply redirects
        for redirect in &redirects {
            if let Err(e) = apply_redirect(redirect) {
                syscalls::write(2, format!("redirect failed: {}\n", e).as_bytes());
                syscalls::exit(1);
            }
        }

        // If not background, set as foreground
        if !background {
            if let Some(tty_fd) = shell.tty_fd {
                let pgid = syscalls::getpgrp() as i32;
                syscalls::tcsetpgrp(tty_fd, pgid);
            }
        }

        // Execute command
        let path = format!("/bin/{}\0", cmd);
        let mut argv = Vec::new();
        
        // Build argv
        for arg in &args {
            let mut arg_cstr = arg.clone();
            arg_cstr.push('\0');
            argv.push(arg_cstr.as_ptr());
        }
        argv.push(core::ptr::null());

        // Empty envp for now
        let envp = [core::ptr::null()];

        syscalls::execve(path.as_bytes(), &argv, &envp);

        // If execve returns, it failed
        syscalls::write(2, format!("mello-sh: {}: command not found\n", cmd).as_bytes());
        syscalls::exit(127);
    } else {
        // Parent process
        let child_pid = pid as i32;

        // Set child's process group from parent side
        syscalls::setpgid(child_pid, child_pid);

        if background {
            // Add to job table
            let job_id = shell.jobs_mut().add_job(child_pid, args.join(" "));
            syscalls::write(1, format!("[{}] {}\n", job_id, child_pid).as_bytes());
            Ok(0)
        } else {
            // Set as foreground group
            if let Some(tty_fd) = shell.tty_fd {
                syscalls::tcsetpgrp(tty_fd, child_pid);
            }

            // Wait for child
            let mut status = 0;
            let result = syscalls::wait4(child_pid, &mut status, syscalls::WUNTRACED);

            // Restore shell as foreground
            if let Some(tty_fd) = shell.tty_fd {
                let shell_pgid = syscalls::getpgrp() as i32;
                syscalls::tcsetpgrp(tty_fd, shell_pgid);
            }

            if result < 0 {
                return Err("wait4 failed".into());
            }

            // Extract exit status
            let exit_status = if status & 0x7f == 0 {
                (status >> 8) & 0xff
            } else {
                128 + (status & 0x7f)
            };

            Ok(exit_status)
        }
    }
}

/// Execute a pipeline
fn execute_pipeline(
    shell: &mut Shell,
    commands: Vec<Command>,
    background: bool,
) -> Result<i32, String> {
    if commands.is_empty() {
        return Err("empty pipeline".into());
    }

    let n = commands.len();
    let mut pipes: Vec<[i32; 2]> = Vec::new();
    let mut pids: Vec<i32> = Vec::new();

    // Create pipes
    for _ in 0..n - 1 {
        let mut pipe_fds = [0, 0];
        if syscalls::pipe(&mut pipe_fds) < 0 {
            return Err("pipe failed".into());
        }
        pipes.push(pipe_fds);
    }

    // Fork children
    for (i, cmd) in commands.iter().enumerate() {
        let pid = syscalls::fork();
        
        if pid < 0 {
            return Err("fork failed".into());
        }

        if pid == 0 {
            // Child process
            
            // Set up stdin from previous pipe
            if i > 0 {
                syscalls::dup2(pipes[i - 1][0], 0);
            }

            // Set up stdout to next pipe
            if i < n - 1 {
                syscalls::dup2(pipes[i][1], 1);
            }

            // Close all pipe fds
            for pipe in &pipes {
                syscalls::close(pipe[0]);
                syscalls::close(pipe[1]);
            }

            // Execute command (simplified - only simple commands in pipeline)
            if let Command::Simple { args, redirects, .. } = cmd {
                // Apply redirects
                for redirect in redirects {
                    if let Err(e) = apply_redirect(redirect) {
                        syscalls::write(2, format!("redirect failed: {}\n", e).as_bytes());
                        syscalls::exit(1);
                    }
                }

                // Execute
                let cmd_name = &args[0];
                let path = format!("/bin/{}\0", cmd_name);
                let mut argv = Vec::new();
                
                for arg in args {
                    let mut arg_cstr = arg.clone();
                    arg_cstr.push('\0');
                    argv.push(arg_cstr.as_ptr());
                }
                argv.push(core::ptr::null());

                let envp = [core::ptr::null()];
                syscalls::execve(path.as_bytes(), &argv, &envp);

                syscalls::write(2, format!("mello-sh: {}: command not found\n", cmd_name).as_bytes());
                syscalls::exit(127);
            } else {
                syscalls::write(2, b"nested pipelines not supported\n");
                syscalls::exit(1);
            }
        } else {
            // Parent - save PID
            pids.push(pid as i32);

            // Set process group (first child becomes group leader)
            if i == 0 {
                syscalls::setpgid(pid as i32, pid as i32);
            } else {
                syscalls::setpgid(pid as i32, pids[0]);
            }
        }
    }

    // Close all pipes in parent
    for pipe in &pipes {
        syscalls::close(pipe[0]);
        syscalls::close(pipe[1]);
    }

    let pgid = pids[0];

    if background {
        // Add to job table
        let job_id = shell.jobs_mut().add_job(pgid, "pipeline".to_string());
        syscalls::write(1, format!("[{}] {}\n", job_id, pgid).as_bytes());
        Ok(0)
    } else {
        // Set as foreground group
        if let Some(tty_fd) = shell.tty_fd {
            syscalls::tcsetpgrp(tty_fd, pgid);
        }

        // Wait for all children
        let mut last_status = 0;
        for _ in 0..n {
            let mut status = 0;
            syscalls::wait4(-1, &mut status, 0);
            last_status = status;
        }

        // Restore shell as foreground
        if let Some(tty_fd) = shell.tty_fd {
            let shell_pgid = syscalls::getpgrp() as i32;
            syscalls::tcsetpgrp(tty_fd, shell_pgid);
        }

        // Extract exit status of last command
        let exit_status = if last_status & 0x7f == 0 {
            (last_status >> 8) & 0xff
        } else {
            128 + (last_status & 0x7f)
        };

        Ok(exit_status)
    }
}

/// Apply a redirect
fn apply_redirect(redirect: &Redirect) -> Result<(), String> {
    let mut path = redirect.target.clone();
    path.push('\0');

    match redirect.kind {
        RedirectKind::Input => {
            let fd = syscalls::open(path.as_bytes(), syscalls::O_RDONLY, 0);
            if fd < 0 {
                return Err("cannot open input file".into());
            }
            syscalls::dup2(fd as i32, 0);
            syscalls::close(fd as i32);
        }
        RedirectKind::Output => {
            let fd = syscalls::open(
                path.as_bytes(),
                syscalls::O_WRONLY | syscalls::O_CREAT | syscalls::O_TRUNC,
                0o644,
            );
            if fd < 0 {
                return Err("cannot open output file".into());
            }
            syscalls::dup2(fd as i32, 1);
            syscalls::close(fd as i32);
        }
        RedirectKind::Append => {
            let fd = syscalls::open(
                path.as_bytes(),
                syscalls::O_WRONLY | syscalls::O_CREAT | syscalls::O_APPEND,
                0o644,
            );
            if fd < 0 {
                return Err("cannot open output file".into());
            }
            syscalls::dup2(fd as i32, 1);
            syscalls::close(fd as i32);
        }
    }

    Ok(())
}
