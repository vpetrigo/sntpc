//! Demonstrates how to make a single NTP request to a NTP server of interest
//!
//! Example provides a basic implementation of [`NtpTimestampGenerator`] and [`NtpUdpSocket`]
//! required for the `sntpc` library
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org:123";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com:123";

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

        let result = sntpc::simple_get_time(POOL_NTP_ADDR, &socket);

        match result {
            Ok(time) => {
                assert_ne!(time.sec(), 0);
                let seconds = time.sec();
                let microseconds = u64::from(time.sec_fraction()) * 1_000_000
                    / u64::from(u32::MAX);
                println!("Got time: {seconds}.{microseconds}");
            }
            Err(err) => println!("Err: {err:?}"),
        }

        thread::sleep(Duration::new(15, 0));
    }
}
