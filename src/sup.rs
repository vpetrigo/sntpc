/// Supplementary module to implement some `sntpc` boilerplate that environments with
/// `std` enable have to re-implement.
use core::fmt::Debug;
use std::net;
use std::time::{self, Duration};

use crate::get_time;
use crate::{NtpContext, NtpResult, NtpTimestampGenerator};

/// Standard library timestamp generator wrapper type
/// that relies on `std::time` to provide timestamps during SNTP client operations
#[derive(Copy, Clone, Default)]
pub struct StdTimestampGen {
    duration: Duration,
}

impl NtpTimestampGenerator for StdTimestampGen {
    fn init(&mut self) {
        self.duration = time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .unwrap();
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

/// Supplementary `get_time` alternative that wraps provided UDP socket into a wrapper type
/// that implements necessary traits for SNTP client proper operation
pub fn simple_get_time<A>(
    pool_addrs: A,
    socket: net::UdpSocket,
) -> crate::Result<NtpResult>
where
    A: net::ToSocketAddrs + Copy + Debug,
{
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    get_time(pool_addrs, socket, ntp_context)
}
