use simple_logger;

const GOOGLE_NTP_ADDR: &str = "time.google.com";

fn main() {
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    let time = sntpc::request(GOOGLE_NTP_ADDR, 123).expect(
        format!("Unable to receive time from: {}", GOOGLE_NTP_ADDR).as_str(),
    );

    sntpc::utils::update_system_time(time.sec(), time.nsec());
}
