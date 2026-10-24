//! Manual daemonization - no external crate dependency
//! Uses classic double-fork technique

use libc::{chdir, close, dup2, fork, open, setsid, umask, O_APPEND, O_CREAT, O_WRONLY};

use crate::{error::Result, utils};

pub fn daemonize() -> Result<()> {
    // First fork
    match unsafe { fork() } {
        -1 => return Err(std::io::Error::last_os_error().into()),
        0 => {} // Child continues
        _ => std::process::exit(0), // Parent exits
    }

    // Create new session
    if unsafe { setsid() } == -1 {
        return Err(std::io::Error::last_os_error().into());
    }

    // Second fork
    match unsafe { fork() } {
        -1 => return Err(std::io::Error::last_os_error().into()),
        0 => {} // Child (daemon) continues
        _ => std::process::exit(0), // First child exits
    }

    // Change working directory to root
    if unsafe { chdir(b"/\0".as_ptr() as _) } == -1 {
        return Err(std::io::Error::last_os_error().into());
    }

    // Reset file mode mask
    unsafe { umask(0) };

    // Close standard file descriptors
    for fd in 0..3 {
        unsafe { close(fd) };
    }

    let running_as_sudo = utils::running_as_sudo();

    // Reopen stdout/stderr to log files
    let (out_path, err_path) = if running_as_sudo {
        (b"/var/log/bustd.out\0".as_ptr(), b"/var/log/bustd.err\0".as_ptr())
    } else {
        (b"/tmp/bustd.out\0".as_ptr(), b"/tmp/bustd.err\0".as_ptr())
    };

    let flags = O_WRONLY | O_CREAT | O_APPEND;
    let mode = 0o644;

    let out_fd = unsafe { open(out_path as _, flags, mode) };
    let err_fd = unsafe { open(err_path as _, flags, mode) };

    if out_fd >= 0 {
        unsafe { dup2(out_fd, 1) };
        if out_fd != 1 { unsafe { close(out_fd) }; }
    }
    if err_fd >= 0 {
        unsafe { dup2(err_fd, 2) };
        if err_fd != 2 { unsafe { close(err_fd) }; }
    }

    // Write PID file - use separate paths to avoid array size issues
    let pid_path = if running_as_sudo {
        b"/var/run/bustd.pid\0".as_ptr()
    } else {
        b"/tmp/bustd.pid\0".as_ptr()
    };
    let pid_fd = unsafe { open(pid_path as _, flags, mode) };
    if pid_fd >= 0 {
        let pid = unsafe { libc::getpid() }.to_string();
        let pid_bytes = pid.as_bytes();
        unsafe { libc::write(pid_fd, pid_bytes.as_ptr() as _, pid_bytes.len()) };
        unsafe { close(pid_fd) };
    }

    println!("[LOG] Daemon started successfully (PID: {})", unsafe { libc::getpid() });
    Ok(())
}