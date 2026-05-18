//! [`NtpTimestampGenerator`] implementation backed by [`embassy_time`] for the [`sntpc`] SNTP client library.
//!
//! This crate provides an [`EmbassyTimestampGenerator`] that uses the Embassy async runtime's
//! monotonic clock to produce timestamps suitable for SNTP round-trip time measurements in
//! embedded `no_std` environments.
//!
//! # Design Rationale
//!
//! The timestamp generator is separated into its own crate to:
//! - **Independent versioning**: Update `embassy-time` without requiring `sntpc` core updates
//! - **Version flexibility**: Works with `embassy-time` 0.5.x (`>=0.5, <0.6`)
//! - **Embedded focus**: Minimal dependencies suitable for `no_std` embedded systems
//! - **Clean separation**: Core SNTP protocol logic remains independent of the async runtime
//!
//! # Important
//!
//! [`EmbassyTimestampGenerator`] provides **monotonic** timestamps based on [`embassy_time::Instant`],
//! not wall-clock time. This is enough for SNTP request/response delay calculations, where
//! the client measures the elapsed time between sending a request and receiving a response.
//! The actual wall-clock offset is computed by the `sntpc` library using the server's timestamps.
//!
//! # Example
//!
//! ```ignore
//! use sntpc::{get_time, NtpContext};
//! use sntpc_time_embassy::EmbassyTimestampGenerator;
//!
//! // Create an NtpContext with the Embassy timestamp generator
//! let ntp_context = NtpContext::new(EmbassyTimestampGenerator::default());
//!
//! // Use with an Embassy UDP socket adapter
//! let result = get_time(server_addr, &socket, ntp_context).await;
//! ```
//!
//! For complete examples, see the [sntpc examples](https://github.com/vpetrigo/sntpc/tree/master/examples/embassy-net).
#![no_std]

use embassy_time::Instant;
use sntpc::NtpTimestampGenerator;

/// Monotonic timestamp generator backed by [`embassy_time::Instant`].
///
/// This type implements [`NtpTimestampGenerator`] by capturing the current monotonic
/// time via [`Instant::now`] on each call to [`init`](NtpTimestampGenerator::init),
/// then reporting the elapsed seconds and sub-second microseconds.
///
/// It does **not** provide wall-clock time. Instead, it provides a monotonic timestamp
/// source suitable for SNTP round-trip delay calculations, where only the relative
/// elapsed time between request and response matters.
///
/// # Example
///
/// ```ignore
/// use sntpc::NtpContext;
/// use sntpc_time_embassy::EmbassyTimestampGenerator;
///
/// let ntp_context = NtpContext::new(EmbassyTimestampGenerator::default());
/// ```
#[derive(Copy, Clone)]
pub struct EmbassyTimestampGenerator {
    instant: Instant,
}

impl Default for EmbassyTimestampGenerator {
    /// Returns a default `EmbassyTimestampGenerator` with the instant set to the epoch.
    ///
    /// The instant is initialized to `Instant::from_secs(0)` and will be updated
    /// to the current time when [`init`](NtpTimestampGenerator::init) is called
    /// before each timestamp measurement.
    fn default() -> Self {
        Self {
            instant: Instant::from_secs(0),
        }
    }
}

impl NtpTimestampGenerator for EmbassyTimestampGenerator {
    /// Captures the current monotonic time from [`embassy_time::Instant::now`].
    ///
    /// This method should be called before each pair of
    /// [`timestamp_sec`](NtpTimestampGenerator::timestamp_sec) and
    /// [`timestamp_subsec_micros`](NtpTimestampGenerator::timestamp_subsec_micros)
    /// calls to ensure the timestamp reflects the current moment.
    fn init(&mut self) {
        self.instant = Instant::now();
    }

    /// Returns the whole seconds component of the captured monotonic instant.
    ///
    /// The value represents seconds elapsed since the Embassy runtime's monotonic
    /// clock epoch, not since the UNIX epoch.
    fn timestamp_sec(&self) -> u64 {
        self.instant.as_secs()
    }

    /// Returns the sub-second microsecond component of the captured monotonic instant.
    ///
    /// This is the fractional part of the timestamp in whole microseconds
    /// (i.e., microseconds within the current second, in the range `0..1_000_000`).
    fn timestamp_subsec_micros(&self) -> u32 {
        (self.instant.as_micros() % 1_000_000) as u32
    }
}
