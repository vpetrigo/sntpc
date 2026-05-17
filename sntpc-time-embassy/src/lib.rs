//! [`NtpTimestampGenerator`] based on [`embassy_time`] for [`sntpc`]
#![no_std]

use embassy_time::Instant;
use sntpc::NtpTimestampGenerator;

/// Timestamp generator backed by `embassy-time`.
///
/// This does not provide wall-clock time. It provides a monotonic timestamp
/// source for `sntpc-net-emabssy`, which is enough for request/response measurements.
#[derive(Copy, Clone)]
pub struct EmbassyTimestampGenerator {
    instant: Instant,
}

impl Default for EmbassyTimestampGenerator {
    fn default() -> Self {
        Self {
            instant: Instant::from_secs(0),
        }
    }
}

impl NtpTimestampGenerator for EmbassyTimestampGenerator {
    fn init(&mut self) {
        self.instant = Instant::now();
    }

    fn timestamp_sec(&self) -> u64 {
        self.instant.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        (self.instant.as_micros() % 1_000_000) as u32
    }
}
