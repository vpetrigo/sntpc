//! Helper utils to synchronize time of a system
//!
//! Currently Unix and Windows based systems are supported
#[cfg(feature = "log")]
use chrono::Timelike;
use chrono::{Local, TimeZone, Utc};
#[cfg(feature = "log")]
use log::debug;

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
    let time = Utc.timestamp_opt(sec as i64, nsec);

    if let Some(time) = time.single() {
        let local_time = time.with_timezone(&Local);
        #[cfg(feature = "log")]
        debug!(
            "UTC time: {:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        );
        #[cfg(feature = "log")]
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
