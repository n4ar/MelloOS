//! Job control for mello-sh

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::syscalls;

/// Job state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobState {
    Running,
    Stopped,
    Done(i32), // Exit status
}

/// Job information
#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub pgid: i32,
    pub command: String,
    pub state: JobState,
}

/// Job table
pub struct JobTable {
    jobs: Vec<Job>,
    next_id: usize,
}

impl JobTable {
    /// Create a new job table
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a new job
    pub fn add_job(&mut self, pgid: i32, command: String) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.jobs.push(Job {
            id,
            pgid,
            command,
            state: JobState::Running,
        });

        id
    }

    /// Get job by ID
    pub fn get_job(&self, id: usize) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    /// Get job by PGID
    pub fn get_job_by_pgid(&self, pgid: i32) -> Option<&Job> {
        self.jobs.iter().find(|j| j.pgid == pgid)
    }

    /// Get mutable job by ID
    pub fn get_job_mut(&mut self, id: usize) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    /// Get all jobs
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Check for completed or stopped jobs
    pub fn check_jobs(&mut self) {
        let mut status = 0;
        
        loop {
            let pid = syscalls::wait4(
                -1,
                &mut status,
                syscalls::WNOHANG | syscalls::WUNTRACED | syscalls::WCONTINUED,
            );

            if pid <= 0 {
                break;
            }

            // Find job with this PID or PGID
            if let Some(job) = self.jobs.iter_mut().find(|j| j.pgid == pid as i32) {
                if status & 0x7f == 0 {
                    // Exited
                    let exit_status = (status >> 8) & 0xff;
                    job.state = JobState::Done(exit_status);
                    syscalls::write(
                        1,
                        format!("[{}]+ Done    {}\n", job.id, job.command).as_bytes(),
                    );
                } else if status & 0xff == 0x7f {
                    // Stopped
                    job.state = JobState::Stopped;
                    syscalls::write(
                        1,
                        format!("[{}]+ Stopped    {}\n", job.id, job.command).as_bytes(),
                    );
                } else if status & 0xffff == 0xffff {
                    // Continued
                    job.state = JobState::Running;
                }
            }
        }

        // Remove completed jobs
        self.jobs.retain(|j| !matches!(j.state, JobState::Done(_)));
    }

    /// Get current job (most recent)
    pub fn current_job(&self) -> Option<&Job> {
        self.jobs.last()
    }

    /// Get current job ID
    pub fn current_job_id(&self) -> Option<usize> {
        self.current_job().map(|j| j.id)
    }
}
