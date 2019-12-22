use log;
use simple_logger;
use sntpc;
use std::{thread, time};

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com";

fn main() {
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    for _ in 0..5 {
        let result = sntpc::request(POOL_NTP_ADDR, 123);

        match result {
            Ok(time) => {
                assert_ne!(time, 0);
                println!("Got time: {}", time);
            }
            Err(err) => println!("Err: {}", err.to_string()),
        }

        thread::sleep(time::Duration::new(15, 0));
    }
}
