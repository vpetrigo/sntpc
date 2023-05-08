//! Demonstrates how to make a single NTP request to a NTP server of interest
//!
//! Example provides a basic implementation of [`NtpTimestampGenerator`] and [`NtpUdpSocket`]
//! required for the `sntpc` library
#[cfg(feature = "log")]
use simple_logger;
use sntpc;
use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use std::{thread, time};

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org:123";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com:123";

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: Duration,
}

impl NtpTimestampGenerator for StdTimestampGen {
    fn init(&mut self) {
        self.duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

#[derive(Debug)]
struct UdpSocketWrapper(UdpSocket);

impl NtpUdpSocket for UdpSocketWrapper {
    fn send_to<T: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize, Error> {
        match self.0.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}

fn main() {
    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    for _ in 0..5 {
        let socket =
            UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Unable to set UDP socket read timeout");

        let sock_wrapper = UdpSocketWrapper(socket);
        let ntp_context = NtpContext::new(StdTimestampGen::default());
        let result = sntpc::get_time(POOL_NTP_ADDR, sock_wrapper, ntp_context);

        match result {
            Ok(time) => {
                assert_ne!(time.sec(), 0);
                let seconds = time.sec();
                let microseconds =
                    time.sec_fraction() as u64 * 1_000_000 / u32::MAX as u64;
                println!("Got time: {}.{}", seconds, microseconds);
            }
            Err(err) => println!("Err: {:?}", err),
        }

        thread::sleep(time::Duration::new(15, 0));
    }
}
