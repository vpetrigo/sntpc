use sntpc::{
    NtpResult, fraction_to_microseconds, fraction_to_milliseconds, fraction_to_nanoseconds, fraction_to_picoseconds,
};

#[test]
fn test_ntp_result() {
    let result1 = NtpResult::new(0, 0, 0, 0, 1, -2);

    assert_eq!(0, result1.sec());
    assert_eq!(0, result1.sec_fraction());
    assert_eq!(0, result1.roundtrip());
    assert_eq!(0, result1.offset());
    assert_eq!(1, result1.stratum());
    assert_eq!(-2, result1.precision());

    let result2 = NtpResult::new(1, 2, 3, 4, 5, -23);

    assert_eq!(1, result2.sec());
    assert_eq!(2, result2.sec_fraction());
    assert_eq!(3, result2.roundtrip());
    assert_eq!(4, result2.offset());
    assert_eq!(5, result2.stratum());
    assert_eq!(-23, result2.precision());

    let result3 = NtpResult::new(u32::MAX - 1, u32::MAX, u64::MAX, i64::MAX, 1, -127);

    assert_eq!(u32::MAX, result3.sec());
    assert_eq!(0, result3.sec_fraction());
    assert_eq!(u64::MAX, result3.roundtrip());
    assert_eq!(i64::MAX, result3.offset());
    assert_eq!(-127, result3.precision());
}

#[test]
fn test_ntp_fraction_overflow_result() {
    let result = NtpResult::new(0, u32::MAX, 0, 0, 1, -19);
    assert_eq!(1, result.sec());
    assert_eq!(0, result.sec_fraction());
    assert_eq!(0, result.roundtrip());
    assert_eq!(0, result.offset());

    let result = NtpResult::new(u32::MAX - 1, u32::MAX, 0, 0, 1, -17);
    assert_eq!(u32::MAX, result.sec());
    assert_eq!(0, result.sec_fraction());
    assert_eq!(0, result.roundtrip());
    assert_eq!(0, result.offset());
}

#[test]
fn test_conversion_to_ms() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
    let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
    assert_eq!(999u32, milliseconds);

    let result = NtpResult::new(0, 0, 0, 0, 1, 0);
    let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
    assert_eq!(0u32, milliseconds);
}

#[test]
fn test_conversion_to_us() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
    let microseconds = fraction_to_microseconds(result.seconds_fraction);
    assert_eq!(999_999u32, microseconds);

    let result = NtpResult::new(0, 0, 0, 0, 1, 0);
    let microseconds = fraction_to_microseconds(result.seconds_fraction);
    assert_eq!(0u32, microseconds);
}

#[test]
fn test_conversion_to_ns() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
    let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
    assert_eq!(999_999_999u32, nanoseconds);

    let result = NtpResult::new(0, 0, 0, 0, 1, 0);
    let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
    assert_eq!(0u32, nanoseconds);
}

#[test]
fn test_conversion_to_ps() {
    let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(999_999_999_767u64, picoseconds);

    let result = NtpResult::new(0, 1, 0, 0, 1, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(232u64, picoseconds);

    let result = NtpResult::new(0, 0, 0, 0, 1, 0);
    let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
    assert_eq!(0u64, picoseconds);
}

// #[test]
// fn test_kiss_of_death() {
//     let defined_codes = [
//         "ACST", "AUTH", "AUTO", "BCST", "CRYP", "DENY", "DROP", "RSTR", "INIT", "MCST", "NKEY", "RATE", "RMOT", "STEP",
//     ];
//
//     for code in defined_codes {
//         let ref_id_bytes = code.as_bytes();
//         let ref_id = u32::from_be_bytes(ref_id_bytes.try_into().unwrap_or([0; 4]));
//         let req_resp = SendRequestResult {
//             originate_timestamp: 0u64,
//             version: 0u8,
//         };
//         let response = NtpPacket {
//             li_vn_mode: 0xC4,
//             stratum: 0,
//             poll: 0,
//             precision: 0,
//             root_delay: 0,
//             root_dispersion: 0,
//             ref_id,
//             ref_timestamp: 0,
//             origin_timestamp: 0,
//             recv_timestamp: 0,
//             tx_timestamp: 0,
//         };
//         let raw: RawNtpPacket = (&response).into();
//         let result = process_response(req_resp, raw, 0);
//
//         assert!(result.is_err());
//
//         match result.unwrap_err() {
//             Error::KissOfDeath(kod_code) => assert_eq!(kod_code.as_str(), code),
//             _ => unreachable!("Unexpected error code"),
//         }
//     }
// }
