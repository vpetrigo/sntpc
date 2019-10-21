use std::process::Command;

use chrono::{DateTime, Datelike, Local, Timelike};

pub(super) fn sync_time(time: DateTime<Local>) {
    let cmd = Command::new("cmd")
        .args(&[
            "/C",
            format!(
                "powershell Set-Date -Date “{}/{}/{} {}:{}:{}",
                time.month(),
                time.day(),
                time.year(),
                time.hour(),
                time.minute(),
                time.second()
            )
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
            eprintln!("Error occurred: {}", e.to_string());
        }
    };
}
