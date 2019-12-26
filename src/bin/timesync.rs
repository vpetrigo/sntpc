const GOOGLE_NTP_ADDR: &str = "time.google.com";

fn main() {
    let time = sntpc::request(GOOGLE_NTP_ADDR, 123).expect(
        format!("Unable to receive time from: {}", GOOGLE_NTP_ADDR).as_str(),
    );

    sntpc::utils::update_system_time(time.sec());
}
