//! Demonstrates how to make a system timesync with OS specific [`utils`]
//!
//! You can run the `timesync` example in the terminal:
//!
//! ```
//! cargo run --example timesync --features="std clap utils"
//! ```
//!
//! That will run the example with the default NTP server set to `time.google.com`. To
//! change a server/port settings options available:
//! - `-p`/`--port` - specify port (default: `123`)
//! - `-s`/`--server` - specify server (default: `time.google.com`)
//!
//! So, command in the terminal with all options involved:
//!
//! ```
//! cargo run --example timesync --features="std clap utils" -- -s pool.ntp.org -p 123
//! ```
//!
//! Example provides a basic implementation of [`NtpTimestampGenerator`] and [`NtpUdpSocket`]
//! required for the [`sntpc`] library
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::FromStr;
use std::time::Duration;

use clap::{crate_version, App, Arg};
#[cfg(feature = "log")]
use simple_logger;
use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket};

const GOOGLE_NTP_ADDR: &str = "time.google.com";

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
    let app = App::new("timesync")
        .version(crate_version!())
        .arg(
            Arg::with_name("server")
                .short("s")
                .long("server")
                .takes_value(true)
                .default_value(GOOGLE_NTP_ADDR)
                .help("NTP server hostname"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .default_value("123")
                .help("NTP server port"),
        )
        .get_matches();

    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    let ntp_server = app.value_of("server").unwrap();
    let ntp_port = u32::from_str(app.value_of("port").unwrap());

    let ntp_port = match ntp_port {
        Ok(ntp_port) => ntp_port,
        Err(err) => {
            eprintln!(
                "Unable to convert NTP server port value: {}",
                err.to_string()
            );
            return;
        }
    };

    let ntp_addr = format!("{}:{}", ntp_server, ntp_port);

    let socket =
        UdpSocket::bind("0.0.0.0:0").expect("Unable to create UDP socket");
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Unable to set UDP socket read timeout");
    let sock_wrapper = UdpSocketWrapper(socket);
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    let time = sntpc::get_time(ntp_addr.as_str(), sock_wrapper, ntp_context)
        .expect(format!("Unable to receive time from: {}", ntp_addr).as_str());

    sntpc::utils::update_system_time(time.sec(), time.nsec());
}
