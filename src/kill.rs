//! Process victim selection and killing - optimized hot path

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, Instant};

use libc::{kill, EINVAL, EPERM, ESRCH, SIGKILL, SIGTERM};

use crate::cli::CommandLineArgs;
use crate::errno::errno;
use crate::error::{Error, Result};
use crate::process::Process;

/// Choose victim process - optimized for minimal allocations and syscalls
pub fn choose_victim(
    proc_buf: &mut [u8],
    buf: &mut [u8],
    args: &CommandLineArgs,
    page_size_kib: i64, // Cached page size
) -> Result<Process> {
    let now = Instant::now();

    // Single pass through /proc - filter and map in one go
    let mut victim: Option<(Process, i64)> = None;

    let read_dir = fs::read_dir("/proc")?;
    for entry in read_dir {
        let Ok(entry) = entry else { continue };
        let pid = parse_pid_entry(entry);
        let Some(pid) = pid else { continue };
        if pid <= 1 { continue; } // Skip kernel threads (PID 0, 1)

        let Ok(process) = Process::from_pid(pid, proc_buf) else { continue };

        // Quick check: if we have a victim with higher oom_score, skip
        if let Some((current, _)) = &victim {
            if current.oom_score > process.oom_score {
                continue;
            }
        }

        // Check unkillable patterns (only if feature enabled)
        // Glob-ignore feature disabled for binary size
        // #[cfg(feature = "glob-ignore")]
        // {
        //     if let Some(patterns) = &args.ignored {
        //         if process.is_unkillable(buf, patterns).unwrap_or(false) {
        //             continue;
        //         }
        //     }
        // }

        // Get RSS - skip kernel threads (RSS == 0)
        let Ok(cur_vm_rss_kib) = process.vm_rss_kib(buf, page_size_kib) else { continue };
        if cur_vm_rss_kib == 0 { continue; }

        // Check oom_score_adj - skip unkillable (-1000)
        let Ok(cur_oom_score_adj) = process.oom_score_adj(buf) else { continue };
        if cur_oom_score_adj == -1000 { continue; }

        // Determine if this process is a better victim
        let should_replace = match &victim {
            None => true,
            Some((current, current_rss)) => {
                process.oom_score > current.oom_score
                    || (process.oom_score == current.oom_score && cur_vm_rss_kib > *current_rss)
            }
        };

        if should_replace {
            victim = Some((process, cur_vm_rss_kib));
        }
    }

    let (victim, _) = victim.ok_or(Error::ProcessNotFound)?;

    if args.verbose {
        let elapsed = now.elapsed().as_secs();
        let comm = victim.comm(buf).unwrap_or("unknown");
        eprintln!("[LOG] Found victim in {}s: pid={}, comm={}, oom_score={}", 
            elapsed, victim.pid, comm, victim.oom_score);
    }

    Ok(victim)
}

#[inline]
pub fn kill_process(pid: i32, signal: i32) -> Result<()> {
    match unsafe { kill(pid, signal) } {
        0 => Ok(()),
        _ => Err(match errno() {
            EINVAL => Error::InvalidSignal,
            EPERM => Error::NoPermission,
            ESRCH => Error::ProcessNotFound,
            _ => Error::UnknownKill,
        }),
    }
}

/// Kill process with SIGTERM then SIGKILL, wait for exit
pub fn kill_and_wait(process: Process, _buf: &mut [u8]) -> Result<bool> {
    let pid = process.pid as i32;
    let _ = kill_process(pid, SIGTERM);

    const HALF_SEC: Duration = Duration::from_millis(500);
    let mut sigkill_sent = false;

    for _ in 0..20 { // Max 10 seconds wait
        std::thread::sleep(HALF_SEC);
        if !process.is_alive() {
            if crate::cli::CommandLineArgs::default().verbose {
                eprintln!("[LOG] Process {} exited", pid);
            }
            return Ok(true);
        }
        if !sigkill_sent {
            let _ = kill_process(pid, SIGKILL);
            sigkill_sent = true;
            if crate::cli::CommandLineArgs::default().verbose {
                eprintln!("[LOG] Escalated to SIGKILL for {}", pid);
            }
        }
    }
    Ok(false)
}

/// Parse PID from directory name - zero allocation
#[inline]
fn parse_pid(name: &std::ffi::OsStr) -> Option<u32> {
    let bytes = name.as_bytes();
    if bytes.is_empty() { return None; }
    
    bytes.iter().try_fold(0_u32, |pid, &byte| {
        let digit = byte.checked_sub(b'0')?;
        if digit > 9 { return None; }
        pid.checked_mul(10)?.checked_add(u32::from(digit))
    })
}

#[inline]
fn parse_pid_entry(entry: fs::DirEntry) -> Option<u32> {
    parse_pid(&entry.file_name())
}

#[cfg(test)]
mod tests {
    use super::parse_pid;
    use std::ffi::OsStr;

    #[test]
    fn parse_valid() {
        assert_eq!(parse_pid(OsStr::new("12345")), Some(12345));
        assert_eq!(parse_pid(OsStr::new("1")), Some(1));
    }

    #[test]
    fn reject_invalid() {
        assert_eq!(parse_pid(OsStr::new("")), None);
        assert_eq!(parse_pid(OsStr::new("self")), None);
        assert_eq!(parse_pid(OsStr::new("12a")), None);
    }
}