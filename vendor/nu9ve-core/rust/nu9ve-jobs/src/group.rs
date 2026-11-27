//! The group leader owns the read end of a lifetime pipe. The supervisor alone
//! owns the close-on-exec write end. EOF kills the whole group, even on SIGKILL.
//!
//! The leader is the current executable re-run with the arguments the app
//! chooses (`leader_args`); the app routes them to [`watch`].
use crate::sys;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

/// Body of the group leader: signal readiness, wait for EOF, kill the group.
pub fn watch() -> i32 {
    sys::install_cancellation_handlers();
    if io::stdout()
        .write_all(b"1")
        .and_then(|()| io::stdout().flush())
        .is_err()
    {
        return 1;
    }
    // No data is sent. An unexpected read error also ends the lease.
    let _ = io::copy(&mut io::stdin().lock(), &mut io::sink());
    sys::kill_group(sys::pid(), sys::SIGKILL);
    1
}

pub struct Group(Child);

impl Group {
    /// Start the leader and make `command` join its group. `leases` are file
    /// descriptors (flocks) only the leader inherits, so they outlive the
    /// supervisor until the group is swept.
    pub fn start(
        command: &mut Command,
        leases: &[&File],
        leader_args: &[&str],
    ) -> io::Result<Self> {
        let descriptors: Vec<_> = leases.iter().map(|lease| lease.as_raw_fd()).collect();
        let mut leader = Command::new(std::env::current_exe()?);
        leader
            .args(leader_args)
            .env_clear()
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // SAFETY: the borrows keep the fds open until spawn returns. The
        // child only invokes async-signal-safe fcntl on these descriptors.
        unsafe {
            leader.pre_exec(move || descriptors.iter().try_for_each(|fd| sys::inherit(*fd)));
        }
        let leader = leader.spawn()?;
        let mut group = Self(leader);
        group
            .0
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing watchdog pipe"))?
            .read_exact(&mut [0])?;
        group.adopt(command);
        Ok(group)
    }

    /// Make another `command` join this group (a pipeline's producer), so a
    /// stop or the lifetime pipe reaches it too.
    pub fn adopt(&self, command: &mut Command) {
        let parent = sys::pid();
        command.process_group(self.id());
        // SAFETY: std sets the process group before pre_exec. The closure only
        // invokes getppid and constructs an OS error on the child side of fork.
        unsafe {
            command.pre_exec(move || {
                if sys::parent_pid() == parent {
                    Ok(())
                } else {
                    Err(io::Error::from(io::ErrorKind::BrokenPipe))
                }
            });
        }
    }

    pub fn id(&self) -> i32 {
        i32::try_from(self.0.id()).unwrap_or(0)
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        // Keep the leader unreaped until the last signal, reserving its PID.
        sys::kill_group(self.id(), sys::SIGKILL);
        let _ = self.0.wait();
    }
}
