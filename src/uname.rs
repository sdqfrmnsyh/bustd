//! uname wrapper - minimal

use core::ffi::CStr;
use core::mem;

use libc::{uname, utsname};

use crate::error::{Error, Result};
use crate::linux_version::LinuxVersion;

pub struct Uname {
    uts: utsname,
}

impl Uname {
    #[inline]
    pub fn new() -> Result<Self> {
        let mut uts: utsname = unsafe { mem::zeroed() };
        if unsafe { uname(&mut uts) } < 0 {
            return Err(Error::UnameFailed);
        }
        Ok(Self { uts })
    }

    #[inline]
    pub fn print_info(&self) -> Result<()> {
        let sysname = unsafe { CStr::from_ptr(self.uts.sysname.as_ptr()) }.to_str()?;
        let hostname = unsafe { CStr::from_ptr(self.uts.nodename.as_ptr()) }.to_str()?;
        let release = unsafe { CStr::from_ptr(self.uts.release.as_ptr()) }.to_str()?;
        let arch = unsafe { CStr::from_ptr(self.uts.machine.as_ptr()) }.to_str()?;

        println!("OS:           {}", sysname);
        println!("Hostname:     {}", hostname);
        println!("Version:      {}", release);
        println!("Architecture: {}", arch);
        Ok(())
    }

    #[inline]
    pub fn parse_version(&self) -> Result<LinuxVersion> {
        let release = unsafe { CStr::from_ptr(self.uts.release.as_ptr()) }.to_str()?;
        LinuxVersion::from_str(release).ok_or(Error::InvalidLinuxVersion)
    }
}