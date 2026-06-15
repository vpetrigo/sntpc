use sntpc::{
    NtpResult, fraction_to_microseconds, fraction_to_milliseconds, fraction_to_nanoseconds, fraction_to_picoseconds,
};

#[test]
fn test_ntp_result() {
    let result1 = NtpResult::new(0, 0, 0, 0, 1, -2, 0, 0, 0, [0; 4], 0, 0, 0);

    assert_eq!(0, result1.sec());
    assert_eq!(0, result1.sec_fraction());
    assert_eq!(0, result1.roundtrip());
    assert_eq!(0, result1.offset());
    assert_eq!(1, result1.stratum());
    assert_eq!(-2, result1.precision());
    assert_eq!(0, result1.leap_indicator());
    assert_eq!(0, result1.root_delay());
    assert_eq!(0, result1.root_dispersion());
    assert_eq!([0; 4], result1.reference_id());
    assert_eq!(0, result1.reference_timestamp());
    assert_eq!(0, result1.poll());

    let result2 = NtpResult::new(1, 2, 3, 4, 5, -23, 1, 0, 0, [0; 4], 0, 0, 0);

    assert_eq!(1, result2.sec());
    assert_eq!(2, result2.sec_fraction());
    assert_eq!(3, result2.roundtrip());
    assert_eq!(4, result2.offset());
    assert_eq!(5, result2.stratum());
    assert_eq!(-23, result2.precision());
    assert_eq!(1, result2.leap_indicator());
    assert_eq!(0, result2.root_delay());
    assert_eq!(0, result2.root_dispersion());
    assert_eq!([0; 4], result2.reference_id());
    assert_eq!(0, result2.reference_timestamp());
    assert_eq!(0, result2.poll());

    let result3 = NtpResult::new(
        u32::MAX - 1,
        u32::MAX,
        u64::MAX,
        i64::MAX,
        1,
        -127,
        2,
        0,
        0,
        [0; 4],
        0,
        0,
        0,
    );

    assert_eq!(u32::MAX - 1, result3.sec());
    assert_eq!(u32::MAX, result3.sec_fraction());
    assert_eq!(u64::MAX, result3.roundtrip());
    assert_eq!(i64::MAX, result3.offset());
    assert_eq!(-127, result3.precision());
    assert_eq!(2, result3.leap_indicator());
}

#[test]
fn test_ntp_fraction_overflow_result() {
    let result = NtpResult::new(0, u32::MAX, 0, 0, 1, -19, 0, 0, 0, [0; 4], 0, 0, 0);
    assert_eq!(0, result.sec());
    assert_eq!(u32::MAX, result.sec_fraction());
    assert_eq!(0, result.roundtrip());
    assert_eq!(0, result.offset());

    let result = NtpResult::new(u32::MAX - 1, u32::MAX, 0, 0, 1, -17, 0, 0, 0, [0; 4], 0, 0, 0);
    assert_eq!(u32::MAX - 1, result.sec());
    assert_eq!(u32::MAX, result.sec_fraction());
    assert_eq!(0, result.roundtrip());
    assert_eq!(0, result.offset());
}

#[test]
fn test_conversion_to_ms() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
    assert_eq!(999u32, milliseconds);
    assert_eq!(999u32, fraction_to_milliseconds(u32::MAX));

    let result = NtpResult::new(0, 0, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
    assert_eq!(0u32, milliseconds);
    assert_eq!(0u32, fraction_to_milliseconds(1));
}

#[test]
fn test_conversion_to_us() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let microseconds = fraction_to_microseconds(result.seconds_fraction);
    assert_eq!(999_999u32, microseconds);

    assert_eq!(999_999u32, fraction_to_microseconds(u32::MAX));

    let result = NtpResult::new(0, 0, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let microseconds = fraction_to_microseconds(result.seconds_fraction);
    assert_eq!(0u32, microseconds);
    assert_eq!(0u32, fraction_to_microseconds(1));
}

#[test]
fn test_conversion_to_ns() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
    assert_eq!(999_999_999u32, nanoseconds);

    assert_eq!(999_999_999u32, fraction_to_nanoseconds(u32::MAX));

    let result = NtpResult::new(0, 0, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
    assert_eq!(0u32, nanoseconds);
    assert_eq!(0u32, fraction_to_nanoseconds(1));
}

#[test]
fn test_conversion_to_ps() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(999_999_999_534u64, picoseconds);

    assert_eq!(999_999_999_767u64, fraction_to_picoseconds(u32::MAX));

    let result = NtpResult::new(0, 1, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(232u64, picoseconds);

    let result = NtpResult::new(0, 0, 0, 0, 1, 0, 0, 0, 0, [0; 4], 0, 0, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(0u64, picoseconds);
}
