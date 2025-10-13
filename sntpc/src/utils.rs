//! Helper utils to synchronize time of a system
//!
//! Currently, Unix and Windows based systems are supported
#[cfg(any(feature = "log", feature = "defmt"))]
use crate::log::debug;
#[cfg(any(feature = "log", feature = "defmt"))]
use chrono::Timelike;
use chrono::{Local, TimeZone, Utc};

#[cfg(unix)]
use unix::sync_time;
#[cfg(windows)]
use windows::sync_time;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Set up system time based on the given parameters
/// Args:
/// * `sec` - Seconds since UNIX epoch start
/// * `nsec` - Fraction of seconds from an NTP response
pub fn update_system_time(sec: u32, nsec: u32) {
    let time = Utc.timestamp_opt(i64::from(sec), nsec);

    if let Some(time) = time.single() {
        let local_time = time.with_timezone(&Local);
        #[cfg(any(feature = "log", feature = "defmt"))]
        debug!("UTC time: {:02}:{:02}:{:02}", time.hour(), time.minute(), time.second());
        #[cfg(any(feature = "log", feature = "defmt"))]
        debug!(
            "{} time: {:02}:{:02}:{:02}",
            local_time.offset(),
            local_time.hour(),
            local_time.minute(),
            local_time.second()
        );

        sync_time(local_time);
    }
}
