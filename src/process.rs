//! Process inspection - zero-allocation, minimal syscalls

use std::io::Read;
use std::io::Write;

use libc::getpgid;

use crate::{error::{Error, Result}, utils::{str_from_bytes, file_from_buffer}};

#[derive(Debug, Default, Clone, Copy)]
pub struct Process {
    pub pid: u32,
    pub oom_score: i16,
}

impl Process {
    #[inline]
    pub fn from_pid(pid: u32, buf: &mut [u8]) -> Result<Self> {
        let oom_score = Self::oom_score_from_pid(pid, buf)?;
        Ok(Self { pid, oom_score })
    }

    #[inline]
    pub fn is_alive_from_pid(pid: u32) -> bool {
        // getpgid returns -1 on error, positive on success
        (unsafe { getpgid(pid as i32) }) > 0
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        Self::is_alive_from_pid(self.pid)
    }

    /// Get current process - used for testing
    #[cfg(test)]
    pub fn this(buf: &mut [u8]) -> Result<Self> {
        let pid = unsafe { libc::getpid() } as u32;
        Self::from_pid(pid, buf)
    }

    /// Read /proc/<pid>/comm - returns trimmed process name
    pub fn comm<'a>(&self, buf: &'a mut [u8]) -> Result<&'a str> {
        write!(&mut *buf, "/proc/{}/comm\0", self.pid)?;
        let mut file = file_from_buffer(buf)?;
        buf.fill(0);
        file.read(buf)?;
        str_from_bytes(buf).map(|s| s.trim())
    }

    /// Read oom_score from /proc/<pid>/oom_score
    #[inline]
    pub fn oom_score_from_pid(pid: u32, buf: &mut [u8]) -> Result<i16> {
        write!(&mut *buf, "/proc/{}/oom_score\0", pid)?;
        let mut file = file_from_buffer(buf)?;
        buf.fill(0);
        file.read(buf)?;
        str_from_bytes(buf)?.trim().parse().map_err(Into::into)
    }

    /// Read VmRSS from /proc/<pid>/statm, convert to KiB using cached page size
    #[inline]
    pub fn vm_rss_kib(&self, buf: &mut [u8], page_size_kib: i64) -> Result<i64> {
        write!(&mut *buf, "/proc/{}/statm\0", self.pid)?;
        let mut file = file_from_buffer(buf)?;
        buf.fill(0);
        file.read(buf)?;

        // Parse second column (RSS pages) directly from bytes
        let data = str_from_bytes(buf)?;
        let mut iter = data.split_ascii_whitespace();
        iter.next(); // skip first column (size)
        let rss_pages: i64 = iter.next().ok_or(Error::MalformedStatm)?.parse()?;
        Ok(rss_pages * page_size_kib)
    }

    /// Read oom_score_adj from /proc/<pid>/oom_score_adj
    #[inline]
    pub fn oom_score_adj(&self, buf: &mut [u8]) -> Result<i16> {
        write!(&mut *buf, "/proc/{}/oom_score_adj\0", self.pid)?;
        let mut file = file_from_buffer(buf)?;
        buf.fill(0);
        file.read(buf)?;
        str_from_bytes(buf)?.trim().parse().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::page_size;

    #[test]
    fn comm() {
        let mut buf = [0_u8; 100];
        let this = Process::this(&mut buf).unwrap();
        let comm = this.comm(&mut buf).unwrap();
        assert!(!comm.is_empty());
    }

    #[test]
    fn oom_score() {
        let mut buf = [0_u8; 100];
        let this = Process::this(&mut buf).unwrap();
        assert!(this.oom_score >= 0);
    }

    #[test]
    fn vm_rss() {
        let mut buf = [0_u8; 100];
        let this = Process::this(&mut buf).unwrap();
        let page_size_kib = page_size().unwrap() / 1024;
        let rss = this.vm_rss_kib(&mut buf, page_size_kib).unwrap();
        assert!(rss > 0);
    }
}