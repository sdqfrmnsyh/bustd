//! Memory info via sysinfo syscall - faster than /proc/meminfo

use std::mem;

use libc::sysinfo;

use crate::{error::{Error, Result}, utils::bytes_to_megabytes};

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryInfo {
    pub total_ram_mb: u64,
    pub total_swap_mb: u64,
    pub available_ram_mb: u64,
    pub available_swap_mb: u64,
    pub available_ram_percent: u8,
    pub available_swap_percent: u8,
}

impl MemoryInfo {
    #[inline]
    pub fn new() -> Result<Self> {
        // Zeroed sysinfo struct
        let mut si: sysinfo = unsafe { mem::zeroed() };

        // syscall - much faster than reading /proc/meminfo
        if unsafe { sysinfo(&mut si) } != 0 {
            return Err(Error::SysInfoFailed);
        }

        // Convert directly - sysinfo uses u64 on 64-bit Linux
        let avail_ram_mb = bytes_to_megabytes(si.freeram as u64, si.mem_unit as u64);
        let total_ram_mb = bytes_to_megabytes(si.totalram as u64, si.mem_unit as u64);
        let total_swap_mb = bytes_to_megabytes(si.totalswap as u64, si.mem_unit as u64);
        let avail_swap_mb = bytes_to_megabytes(si.freeswap as u64, si.mem_unit as u64);

        let ram_pct = if total_ram_mb != 0 {
            ((avail_ram_mb as f32 / total_ram_mb as f32) * 100.0) as u8
        } else { 0 };

        let swap_pct = if total_swap_mb != 0 {
            ((avail_swap_mb as f32 / total_swap_mb as f32) * 100.0) as u8
        } else { 0 };

        Ok(Self {
            total_ram_mb,
            available_ram_mb: avail_ram_mb,
            total_swap_mb,
            available_swap_mb: avail_swap_mb,
            available_ram_percent: ram_pct,
            available_swap_percent: swap_pct,
        })
    }
}

impl core::fmt::Display for MemoryInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, 
            "RAM: {}/{} MB ({}%) | Swap: {}/{} MB ({}%)",
            self.available_ram_mb, self.total_ram_mb, self.available_ram_percent,
            self.available_swap_mb, self.total_swap_mb, self.available_swap_percent
        )
    }
}