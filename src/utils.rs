//! Utility functions - zero dependencies, minimal overhead

use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use libc::{sysconf, _SC_PAGESIZE, geteuid};

use crate::error::{Error, Result};
use memchr::memchr;

/// Check if running as root
#[inline]
pub fn running_as_sudo() -> bool {
    unsafe { geteuid() == 0 }
}

/// Get system page size in bytes (cached after first call)
#[allow(dead_code)]
pub fn page_size() -> Result<i64> {
    let ps = unsafe { sysconf(_SC_PAGESIZE) };
    if ps == -1 { return Err(Error::SysConfFailed); }
    Ok(ps.into())
}

/// Convert bytes to megabytes using mem_unit from sysinfo
#[inline]
pub fn bytes_to_megabytes(bytes: u64, mem_unit: u64) -> u64 {
    const B_TO_MB: u64 = 1_000_000;
    bytes / B_TO_MB * mem_unit
}

/// Find first null byte and return slice up to it
#[inline]
pub fn bytes_until_nul(buf: &[u8]) -> &[u8] {
    let idx = memchr(0, buf).unwrap_or(buf.len());
    &buf[..idx]
}

/// Convert buffer to &str (stops at first null)
#[inline]
pub fn str_from_bytes(buf: &[u8]) -> Result<&str> {
    Ok(core::str::from_utf8(bytes_until_nul(buf))?)
}

/// Open file from buffer containing null-terminated path
#[inline]
pub fn file_from_buffer(buf: &[u8]) -> Result<File> {
    let path = Path::new(OsStr::from_bytes(bytes_until_nul(buf)));
    File::open(path).map_err(Into::into)
}