//! Main monitoring loop - optimized for minimal CPU wakeups

use std::time::Duration;

use crate::cli::CommandLineArgs;
use crate::error::Result;
use crate::kill;
use crate::memory;
use crate::memory::MemoryInfo;
use crate::process::Process;

#[derive(Clone, Copy)]
enum MemoryStatus {
    NearTerminal(f32),
    Okay,
}

pub struct Monitor {
    memory_info: MemoryInfo,
    proc_buf: [u8; 50],
    buf: [u8; 100],
    status: MemoryStatus,
    args: CommandLineArgs,
    page_size_kib: i64, // Cached page size in KiB
}

impl Monitor {
    /// Adaptive sleep calculation - copied from earlyoom, tuned for low-spec
    /// Increased MIN_SLEEP to reduce CPU wakeups on weak hardware
    pub fn sleep_time_ms(&self) -> Duration {
        // Conservative fill rate estimates (KiB/s)
        const RAM_FILL_RATE: i64 = 6000;
        const SWAP_FILL_RATE: i64 = 800;

        // Sleep bounds in ms - INCREASED for weak CPU
        const MIN_SLEEP: i64 = 500;  // Was 100 - 5x less CPU wakeups
        const MAX_SLEEP: i64 = 2000; // Was 1000

        // Terminal thresholds (%)
        const RAM_TERMINAL_PERCENT: f64 = 10.;
        const SWAP_TERMINAL_PERCENT: f64 = 10.;

        let ram_headroom_kib = (self.memory_info.available_ram_percent as f64 - RAM_TERMINAL_PERCENT)
            * (self.memory_info.total_ram_mb as f64 * 10.0);
        let swap_headroom_kib = (self.memory_info.available_swap_percent as f64 - SWAP_TERMINAL_PERCENT)
            * (self.memory_info.total_swap_mb as f64 * 10.0);

        let ram_headroom_kib = i64::max(ram_headroom_kib as i64, 0);
        let swap_headroom_kib = i64::max(swap_headroom_kib as i64, 0);

        let time_to_sleep = ram_headroom_kib / RAM_FILL_RATE + swap_headroom_kib / SWAP_FILL_RATE;
        let time_to_sleep = i64::min(time_to_sleep, MAX_SLEEP);
        let time_to_sleep = i64::max(time_to_sleep, MIN_SLEEP);

        Duration::from_millis(time_to_sleep as u64)
    }

    pub fn new(proc_buf: [u8; 50], buf: [u8; 100], args: CommandLineArgs) -> Result<Self> {
        let memory_info = MemoryInfo::new()?;
        let page_size_kib = utils::page_size()? / 1024; // Cache in KiB

        let status = if memory_info.available_ram_percent <= 15 {
            MemoryStatus::NearTerminal(memory::pressure::pressure_some_avg10(&mut [0_u8; 100])?)
        } else {
            MemoryStatus::Okay
        };

        Ok(Self {
            memory_info,
            proc_buf,
            buf,
            status,
            args,
            page_size_kib,
        })
    }

    #[inline]
    fn memory_is_low(&self) -> bool {
        matches!(self.status, MemoryStatus::NearTerminal(psi) if psi >= self.args.cutoff_psi)
    }

    #[inline]
    fn get_victim(&mut self) -> Result<Process> {
        kill::choose_victim(&mut self.proc_buf, &mut self.buf, &self.args, self.page_size_kib)
    }

    fn update_memory_stats(&mut self) -> Result<()> {
        self.memory_info = memory::MemoryInfo::new()?;
        self.status = if self.memory_info.available_ram_percent <= 15 {
            let psi = memory::pressure::pressure_some_avg10(&mut self.buf)?;
            MemoryStatus::NearTerminal(psi)
        } else {
            MemoryStatus::Okay
        };
        Ok(())
    }

    fn free_up_memory(&mut self) -> Result<()> {
        let victim = self.get_victim()?;
        self.update_memory_stats()?;
        if self.memory_is_low() {
            kill::kill_and_wait(victim, &mut self.buf)?;
        }
        Ok(())
    }

    // Hot loop - minimal allocations, maximal inlining
    pub fn poll(&mut self) -> Result<()> {
        loop {
            self.update_memory_stats()?;
            if self.memory_is_low() {
                self.free_up_memory()?;
            }

            let sleep_time = self.sleep_time_ms();
            if self.args.verbose {
                eprintln!("[adaptive-sleep] {}ms", sleep_time.as_millis());
            }

            std::thread::sleep(sleep_time);
        }
    }
}

// Re-export utils for page_size
mod utils {
    use libc::{sysconf, _SC_PAGESIZE};
    use crate::error::{Error, Result};

    pub fn page_size() -> Result<i64> {
        let ps = unsafe { sysconf(_SC_PAGESIZE) };
        if ps == -1 { return Err(Error::SysConfFailed); }
        Ok(ps.into())
    }
}