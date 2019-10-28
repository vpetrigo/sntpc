use std::str::FromStr;

use clap::{crate_version, App, Arg};
use simple_logger;
use std::error::Error;

const GOOGLE_NTP_ADDR: &str = "time.google.com";

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
                err.description()
            );
            return;
        }
    };

    let time = sntpc::request(ntp_server, ntp_port).expect(
        format!("Unable to receive time from: {}", GOOGLE_NTP_ADDR).as_str(),
    );

    sntpc::utils::update_system_time(time.sec(), time.nsec());
}
