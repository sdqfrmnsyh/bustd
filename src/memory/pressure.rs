//! PSI (Pressure Stall Information) reading - minimal allocation

use std::fs::File;
use std::io::Read;

use crate::error::{Error, Result};
use crate::utils::str_from_bytes;

const PSI_PATH: &str = "/proc/pressure/memory";

/// Read `some avg10` from /proc/pressure/memory
/// Returns percentage of time tasks were stalled on memory (0-100)
pub fn pressure_some_avg10(buf: &mut [u8]) -> Result<f32> {
    let mut file = File::open(PSI_PATH)?;
    buf.fill(0);
    file.read(buf)?;
    
    let line = str_from_bytes(buf)?
        .lines()
        .next()
        .ok_or(Error::MalformedPressureFile)?;
    
    // Parse: "some avg10=X.XX avg60=X.XX avg300=X.XX total=X"
    // Find "avg10=" and parse the float after it
    let Some(pos) = line.find("avg10=") else {
        return Err(Error::MalformedPressureFile);
    };
    
    let val_str = &line[pos + 6..];
    let Some(end) = val_str.find(' ').or_else(|| val_str.find('\n')) else {
        return Err(Error::MalformedPressureFile);
    };
    
    val_str[..end].trim().parse().map_err(Into::into)
}