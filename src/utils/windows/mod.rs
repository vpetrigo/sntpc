use super::get_current_timezone_time;
use super::{TimeValue, TimeZone};

use log::debug;

use std::process::Command;
use std::str;
use std::str::FromStr;
use std::vec::Vec;

pub fn update_system_time(seconds: u32) {
    let hours = seconds / 3600 % 24;
    let minutes = seconds / 60 % 60;
    let seconds = seconds % 60;
    debug!("UTC time: {:02}:{:02}:{:02}", hours, minutes, seconds);
    let timezone = get_system_tz();

    let current_tz_time =
        get_current_timezone_time(&timezone, hours, minutes, seconds);
    debug!(
        "Current timezone time (UTC{:+02}:{:02}:{:02}): {:02}:{:02}:{:02}",
        timezone.hours,
        timezone.minutes,
        timezone.seconds,
        current_tz_time.hours,
        current_tz_time.minutes,
        current_tz_time.seconds
    );

    sync_time(current_tz_time);
}

fn sync_time(time: TimeValue) {
    let cmd = Command::new("cmd")
        .args(&[
            "/C",
            format!("time {}:{}:{}", time.hours, time.minutes, time.seconds)
                .as_str(),
        ])
        .spawn();

    match cmd {
        Ok(mut child) => {
            child
                .wait()
                .expect("Time synchronization finished incorrectly");
        }
        Err(e) => {
            println!("Error occured: {}", e.to_string());
        }
    };
}

fn get_system_tz() -> TimeZone {
    let output = Command::new("cmd")
        .args(&["/C", "powershell gtz | findstr BaseUtcOffset"])
        .output()
        .expect("Unable to get UTC base offset");
    let utc_offset = str::from_utf8(output.stdout.as_ref())
        .expect("Unable to convert Windows cmd results");

    let split: Vec<_> = utc_offset.splitn(2, ":").collect();
    assert_eq!(2, split.len());
    let offset: Vec<_> = split[1].trim().split(":").collect();
    assert_eq!(3, offset.len());
    let result: Vec<i32> =
        offset.iter().map(|x| i32::from_str(x).unwrap()).collect();
    assert_eq!(3, result.len());

    result.into()
}
