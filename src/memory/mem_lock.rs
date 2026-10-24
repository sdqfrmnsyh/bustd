//! Memory locking via mlockall - prevents daemon from being swapped

use libc::{c_int, mlockall, EAGAIN, EINVAL, ENOMEM, EPERM, MCL_CURRENT, MCL_FUTURE};

use crate::errno::errno;
use crate::error::{Error, Result};

extern "C" {
    static _MCL_ONFAULT: c_int;
}

#[inline]
fn mlockall_wrapper(flags: c_int) -> Result<()> {
    match unsafe { mlockall(flags) } {
        0 => Ok(()),
        _ => Err(match errno() {
            EAGAIN => Error::CouldNotLockMemory,
            EINVAL => Error::InvalidFlags,
            ENOMEM => Error::TooMuchMemoryToLock,
            EPERM => Error::NoPermission,
            _ => Error::UnknownMlockall,
        }),
    }
}

pub fn lock_memory_pages() -> Result<()> {
    // Try with MCL_ONFAULT first (lazy locking, better for low memory)
    #[allow(non_snake_case)]
    let MCL_ONFAULT: c_int = unsafe { _MCL_ONFAULT };
    
    if mlockall_wrapper(MCL_CURRENT | MCL_FUTURE | MCL_ONFAULT).is_ok() {
        return Ok(());
    }
    
    // Fallback without MCL_ONFAULT
    mlockall_wrapper(MCL_CURRENT | MCL_FUTURE)
}