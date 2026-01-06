//! Demonstrates how to make a system timesync with OS specific [`sntpc::utils`]
//!
//! You can run the `timesync` example in the terminal:
//!
//! ```
//! cargo run --example timesync --features="std utils"
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
use sntpc::{sync::get_time, NtpContext, StdTimestampGen};
use sntpc_net_std::UdpSocketWrapper;

use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

use clap::Parser;

const GOOGLE_NTP_ADDR: &str = "time.google.com";

#[derive(Parser)]
#[command(name = "timesync")]
#[command(version)]
struct Cli {
    /// NTP server hostname
    #[arg(short, long, default_value = GOOGLE_NTP_ADDR)]
    server: String,

    /// NTP server port
    #[arg(short, long, default_value = "123")]
    port: u32,
}

fn main() {
    let cli = Cli::parse();

    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    let ntp_server = &cli.server;
    let ntp_port = cli.port;

    let ntp_addr = format!("{ntp_server}:{ntp_port}");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to create UDP socket");
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Unable to set UDP socket read timeout");
    let socket = UdpSocketWrapper::new(socket);

    for addr in ntp_addr.to_socket_addrs().unwrap() {
        let ntp_context = NtpContext::new(StdTimestampGen::default());
        let result =
            get_time(addr, &socket, ntp_context).unwrap_or_else(|_| panic!("Unable to receive time from: {ntp_addr}"));

        println!("Received time: {result:?}");
        sntpc::utils::update_system_time(result.sec(), result.sec_fraction());
    }
}
