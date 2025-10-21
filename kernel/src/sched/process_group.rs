//! Process Groups and Sessions
//!
//! This module implements process groups and sessions for job control.
//! Process groups allow related processes (like a pipeline) to be managed together.
//! Sessions group process groups that share a controlling terminal.

/// Process ID type
pub type Pid = usize;

/// Process Group ID type (same as Pid)
pub type Pgid = usize;

/// Session ID type (same as Pid)
pub type Sid = usize;

/// Device ID type for controlling terminal
pub type DeviceId = usize;

/// Maximum processes per group
const MAX_PROCESSES_PER_GROUP: usize = 64;

/// Process group structure
///
/// A process group is a collection of related processes that can be
/// signaled together. Typically used for pipelines.
#[derive(Debug, Clone)]
pub struct ProcessGroup {
    /// Process group ID (usually the PID of the group leader)
    pub pgid: Pgid,
    /// List of process IDs in this group
    pub processes: [Option<Pid>; MAX_PROCESSES_PER_GROUP],
    /// Number of processes in this group
    pub process_count: usize,
    /// Session this group belongs to
    pub session: Sid,
}

impl ProcessGroup {
    /// Create a new process group
    pub const fn new(pgid: Pgid, session: Sid) -> Self {
        Self {
            pgid,
            processes: [None; MAX_PROCESSES_PER_GROUP],
            process_count: 0,
            session,
        }
    }

    /// Add a process to this group
    pub fn add_process(&mut self, pid: Pid) -> bool {
        // Check if already in group
        for i in 0..self.process_count {
            if self.processes[i] == Some(pid) {
                return true;
            }
        }

        // Add if space available
        if self.process_count < MAX_PROCESSES_PER_GROUP {
            self.processes[self.process_count] = Some(pid);
            self.process_count += 1;
            true
        } else {
            false
        }
    }

    /// Remove a process from this group
    pub fn remove_process(&mut self, pid: Pid) -> bool {
        for i in 0..self.process_count {
            if self.processes[i] == Some(pid) {
                // Shift remaining processes
                for j in i..self.process_count - 1 {
                    self.processes[j] = self.processes[j + 1];
                }
                self.processes[self.process_count - 1] = None;
                self.process_count -= 1;
                return true;
            }
        }
        false
    }

    /// Check if this group is empty
    pub fn is_empty(&self) -> bool {
        self.process_count == 0
    }

    /// Get the number of processes in this group
    pub fn len(&self) -> usize {
        self.process_count
    }

    /// Get an iterator over process IDs
    pub fn iter(&self) -> impl Iterator<Item = Pid> + '_ {
        self.processes[..self.process_count]
            .iter()
            .filter_map(|&p| p)
    }
}

/// Maximum process groups per session
const MAX_PROCESS_GROUPS_PER_SESSION: usize = 32;

/// Session structure
///
/// A session is a collection of process groups that share a controlling terminal.
/// The session leader is the process that created the session.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID (usually the PID of the session leader)
    pub sid: Sid,
    /// Controlling terminal device (if any)
    pub controlling_tty: Option<DeviceId>,
    /// Foreground process group ID (if any)
    pub foreground_pgid: Option<Pgid>,
    /// List of process group IDs in this session
    pub process_groups: [Option<Pgid>; MAX_PROCESS_GROUPS_PER_SESSION],
    /// Number of process groups in this session
    pub group_count: usize,
}

impl Session {
    /// Create a new session
    pub const fn new(sid: Sid) -> Self {
        Self {
            sid,
            controlling_tty: None,
            foreground_pgid: None,
            process_groups: [None; MAX_PROCESS_GROUPS_PER_SESSION],
            group_count: 0,
        }
    }

    /// Add a process group to this session
    pub fn add_process_group(&mut self, pgid: Pgid) -> bool {
        // Check if already in session
        for i in 0..self.group_count {
            if self.process_groups[i] == Some(pgid) {
                return true;
            }
        }

        // Add if space available
        if self.group_count < MAX_PROCESS_GROUPS_PER_SESSION {
            self.process_groups[self.group_count] = Some(pgid);
            self.group_count += 1;
            true
        } else {
            false
        }
    }

    /// Remove a process group from this session
    pub fn remove_process_group(&mut self, pgid: Pgid) -> bool {
        for i in 0..self.group_count {
            if self.process_groups[i] == Some(pgid) {
                // Shift remaining groups
                for j in i..self.group_count - 1 {
                    self.process_groups[j] = self.process_groups[j + 1];
                }
                self.process_groups[self.group_count - 1] = None;
                self.group_count -= 1;
                return true;
            }
        }
        false
    }

    /// Set the controlling terminal
    pub fn set_controlling_tty(&mut self, tty: DeviceId) {
        self.controlling_tty = Some(tty);
    }

    /// Clear the controlling terminal
    pub fn clear_controlling_tty(&mut self) {
        self.controlling_tty = None;
        self.foreground_pgid = None;
    }

    /// Set the foreground process group
    pub fn set_foreground_pgid(&mut self, pgid: Pgid) {
        self.foreground_pgid = Some(pgid);
    }

    /// Check if a process group is in the foreground
    pub fn is_foreground(&self, pgid: Pgid) -> bool {
        self.foreground_pgid == Some(pgid)
    }

    /// Check if this session has a controlling terminal
    pub fn has_controlling_tty(&self) -> bool {
        self.controlling_tty.is_some()
    }
}
