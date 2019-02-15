use sntp_client;
use std::{thread, time};

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com";

fn main() {
    for _ in 0..4000 {
        let result = sntp_client::request(POOL_NTP_ADDR, 123);

        match result {
            Ok(time) => {
                assert_ne!(time, 0);
                println!("Got time: {}", time);
            }
            Err(err) => println!("Err: {}", err.to_string()),
        }

        thread::sleep(time::Duration::new(1, 0));
    }
}
