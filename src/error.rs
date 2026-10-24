//! Error types - minimal, no std::error::Error impl overhead

use std::str::Utf8Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    UnameFailed,
    ProcessNotFound,
    InvalidSignal,
    Io,
    NoPermission,
    CouldNotLockMemory,
    TooMuchMemoryToLock,
    InvalidFlags,
    UnknownMlockall,
    UnknownKill,
    InvalidLinuxVersion,
    MalformedStatm,
    MalformedPressureFile,
    ParseInt,
    ParseFloat,
    SysConfFailed,
    SysInfoFailed,
    Utf8,
}

pub type Result<T> = core::result::Result<T, Error>;

// From impls for automatic conversion
impl From<std::io::Error> for Error {
    #[inline] fn from(_: std::io::Error) -> Self { Error::Io }
}

impl From<std::num::ParseIntError> for Error {
    #[inline] fn from(_: std::num::ParseIntError) -> Self { Error::ParseInt }
}

impl From<std::num::ParseFloatError> for Error {
    #[inline] fn from(_: std::num::ParseFloatError) -> Self { Error::ParseFloat }
}

impl From<Utf8Error> for Error {
    #[inline] fn from(_: Utf8Error) -> Self { Error::Utf8 }
}