#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

struct TimeZone {
    hours: i32,
    minutes: i32,
    seconds: i32,
}

impl TimeZone {
    fn new(hours: i32, minutes: i32, seconds: i32) -> Self {
        assert!(
            hours >= -12 && hours <= 12,
            "Incorrect hour TZ offset {}",
            hours
        );
        assert!(
            minutes >= 0 && minutes <= 59,
            "Incorrect minute TZ offset {}",
            minutes
        );
        assert!(
            seconds >= 0 && seconds <= 59,
            "Incorrect second TZ offset {}",
            seconds
        );

        TimeZone {
            hours,
            minutes,
            seconds,
        }
    }
}

impl From<Vec<i32>> for TimeZone {
    fn from(val: Vec<i32>) -> Self {
        assert_eq!(3, val.len());
        TimeZone::new(val[0], val[1], val[2])
    }
}

struct TimeValue {
    hours: u32,
    minutes: u32,
    seconds: u32,
}

fn get_current_timezone_time(
    timezone: &TimeZone,
    hours: u32,
    minutes: u32,
    seconds: u32,
) -> TimeValue {
    let result: TimeValue = TimeValue {
        hours: hours_to_timezone(hours, timezone.hours),
        minutes: minutes_to_timezone(minutes, timezone.minutes),
        seconds: seconds_to_timezone(seconds, timezone.seconds),
    };

    result
}

fn hours_to_timezone(hours: u32, tz_hours: i32) -> u32 {
    (((24 + hours) as i32 + tz_hours) % 24) as u32
}

fn minutes_to_timezone(minutes: u32, tz_min: i32) -> u32 {
    ((minutes as i32 + tz_min) % 60) as u32
}

fn seconds_to_timezone(seconds: u32, tz_sec: i32) -> u32 {
    ((seconds as i32 + tz_sec) % 60) as u32
}
