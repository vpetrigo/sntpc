use std::process::Command;

use chrono::{DateTime, Datelike, Local, Timelike};

/// Synchronize system time with the platform specific
/// command line tool
pub(super) fn sync_time(time: DateTime<Local>) {
    let time_str = format!(
        "{}/{}/{} {:02}:{:02}:{:02}",
        time.month(),
        time.day(),
        time.year(),
        time.hour(),
        time.minute(),
        time.second()
    );
    let sync_cmd_status = Command::new("date")
        .args(["-s", time_str.as_str()])
        .status()
        .expect("Unable to execute date command");

    if !sync_cmd_status.success() {
        eprintln!(
            "Date command exit status {}",
            sync_cmd_status.code().unwrap()
        );
    }
}
