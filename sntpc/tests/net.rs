mod helpers;

use helpers::*;
use sntpc::{Error, KissOfDeathCode, NtpContext, sntp_process_response, sntp_send_request};

use core::net::SocketAddr;

#[test]
fn test_send_request_net_errors() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();

    {
        let mut socket = MockUdpSocket::new(dest, [0u8; 48]);
        let context = NtpContext::new(MockTimestampGen);

        socket.update_write_result(Ok(48 / 2));
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(sntp_send_request(dest, &socket, context));

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::Network)));
    }

    {
        let mut socket = MockUdpSocket::new(dest, [0u8; 48]);
        let context = NtpContext::new(MockTimestampGen);

        socket.update_write_result(Err(Error::Network));
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(sntp_send_request(dest, &socket, context));

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::Network)));
    }

    {
        let socket = MockUdpSocket::new(dest, [0u8; 48]);
        let context = NtpContext::new(MockTimestampGen);
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(sntp_send_request(dest, &socket, context));

        assert!(result.is_ok());
    }
}

#[test]
fn test_process_request_net_read_error() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();

    let mut socket = MockUdpSocket::new(dest, [0u8; 48]);
    let context = NtpContext::new(MockTimestampGen);

    socket.update_write_result(Ok(48));
    socket.update_read_result(Err(Error::Network));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::Network)));
}

#[test]
fn test_process_address_mismatch() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let src = "127.0.0.2:123".parse::<SocketAddr>().unwrap();
    let mut socket = MockUdpSocket::new(dest, [0u8; 48]);
    let context = NtpContext::new(MockTimestampGen);

    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, src)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::ResponseAddressMismatch)));
}

#[test]
fn test_process_incorrect_payload() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let mut socket = MockUdpSocket::new(dest, [0u8; 48]);
    let context = NtpContext::new(MockTimestampGen);

    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48 / 2, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::IncorrectPayload)));
}

#[test]
fn test_process_exact_48_byte_response() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    let data = {
        const TS: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];
        data[0] = 0x24;
        data[1] = 1;
        data[4..8].copy_from_slice(&0x0001_0203u32.to_be_bytes());
        data[8..12].copy_from_slice(&0x0004_0506u32.to_be_bytes());
        data[12..16].copy_from_slice(b"GPS ");
        data[16..24].copy_from_slice(&0x1112_1314_1516_1718u64.to_be_bytes());
        data[24..32].copy_from_slice(&TS.to_be_bytes());
        data[32..40].copy_from_slice(&0x2122_2324_2526_2728u64.to_be_bytes());
        data[40..48].copy_from_slice(&TS.to_be_bytes());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;
        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_ok());
}

#[test]
fn test_process_response_with_trailing_bytes() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    let mut data = Vec::from([0u8; 48]);
    data[0] = 0x24;
    data[1] = 1;
    data[24..32].copy_from_slice(&9_487_534_653_230_284_800u64.to_be_bytes());
    data[40..48].copy_from_slice(&9_487_534_653_230_284_800u64.to_be_bytes());
    data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((52, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;
        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_ok());
}

#[test]
fn test_endian_correct_parsing() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    let data = {
        const TS: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];
        data[0] = 0x24;
        data[1] = 1;
        data[2] = 0xFAu8;
        data[3] = 0xFCu8;
        data[4..8].copy_from_slice(&0x0001_0203u32.to_be_bytes());
        data[8..12].copy_from_slice(&0x0004_0506u32.to_be_bytes());
        data[12..16].copy_from_slice(b"TEST");
        data[16..24].copy_from_slice(&0x1112_1314_1516_1718u64.to_be_bytes());
        data[24..32].copy_from_slice(&TS.to_be_bytes());
        data[32..40].copy_from_slice(&0x2122_2324_2526_2728u64.to_be_bytes());
        data[40..48].copy_from_slice(&TS.to_be_bytes());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;
        sntp_process_response(dest, &socket, context, resp.unwrap())
            .await
            .unwrap()
    });

    assert_eq!(0x0001_0203, result.root_delay());
    assert_eq!(0x0004_0506, result.root_dispersion());
    assert_eq!(*b"TEST", result.reference_id());
}

#[test]
fn test_process_incorrect_mode() {
    const PROPER_SNTP_MODE: u8 = 4;
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    // All modes except 4 should be rejected (broadcast mode 5 is also rejected)
    for i in (0..=0b111u8).filter(|&i| i != PROPER_SNTP_MODE) {
        let data = {
            const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
            let mut data = [0u8; 48];

            data[0] = i;
            data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
            data
        };

        let mut socket = MockUdpSocket::new(dest, data);
        socket.update_write_result(Ok(48));
        socket.update_read_result(Ok((48, dest)));
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(async {
            let resp = sntp_send_request(dest, &socket, context).await;

            sntp_process_response(dest, &socket, context, resp.unwrap()).await
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::IncorrectMode)));
    }
}

#[test]
fn test_process_incorrect_response_version() {
    const PROPER_VERSION: u8 = 4;
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    for i in (0..=0b111u8).filter(|&i| i != PROPER_VERSION) {
        let data = {
            const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
            let mut data = [0u8; 48];

            data[0] = 4 | (i << 3);
            data[1] = 1; // non-zero stratum to avoid KoD check
            data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
            data
        };

        let mut socket = MockUdpSocket::new(dest, data);
        socket.update_write_result(Ok(48));
        socket.update_read_result(Ok((48, dest)));
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(async {
            let resp = sntp_send_request(dest, &socket, context).await;

            sntp_process_response(dest, &socket, context, resp.unwrap()).await
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::IncorrectResponseVersion)));
    }
}

#[test]
fn test_process_incorrect_stratum() {
    const DEFINED_STRATUMS: u8 = 16;
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    for i in (0..255u8).filter(|&i| i >= DEFINED_STRATUMS) {
        let data = {
            const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
            let mut data = [0u8; 48];

            data[0] = 4 | (4 << 3);
            data[1] = i;
            data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
            data
        };

        let mut socket = MockUdpSocket::new(dest, data);
        socket.update_write_result(Ok(48));
        socket.update_read_result(Ok((48, dest)));
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let result = executor.block_on(async {
            let resp = sntp_send_request(dest, &socket, context).await;

            sntp_process_response(dest, &socket, context, resp.unwrap()).await
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(Error::IncorrectStratumHeaders)));
    }
}

#[test]
fn test_kiss_of_death() {
    let defined_codes: [(&str, KissOfDeathCode); 14] = [
        ("ACST", KissOfDeathCode::Acst),
        ("AUTH", KissOfDeathCode::Auth),
        ("AUTO", KissOfDeathCode::Auto),
        ("BCST", KissOfDeathCode::Bcst),
        ("CRYP", KissOfDeathCode::Cryp),
        ("DENY", KissOfDeathCode::Deny),
        ("DROP", KissOfDeathCode::Drop),
        ("RSTR", KissOfDeathCode::Rstr),
        ("INIT", KissOfDeathCode::Init),
        ("MCST", KissOfDeathCode::Mcst),
        ("NKEY", KissOfDeathCode::Nkey),
        ("RATE", KissOfDeathCode::Rate),
        ("RMOT", KissOfDeathCode::Rmot),
        ("STEP", KissOfDeathCode::Step),
    ];

    for &(code_str, expected_variant) in &defined_codes {
        let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
        let data = {
            const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
            let mut data = [0u8; 48];

            data[0] = 0x24; // LI=0, VN=4, mode=4
            data[12..16].copy_from_slice(code_str.as_bytes());
            data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
            data[40..48].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
            data
        };

        let mut socket = MockUdpSocket::new(dest, data);

        socket.update_read_result(Ok((data.len(), dest)));

        let context = NtpContext::new(MockTimestampGen);
        let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
        let mut handler: miniloop::task::Handle<()> = miniloop::task::Handle::default();
        let mut task = miniloop::task::Task::new("test", async {
            let result = sntp_send_request(dest, &socket, context).await;
            assert!(result.is_ok());
            let result = sntp_process_response(dest, &socket, context, result.unwrap()).await;

            match result.unwrap_err() {
                Error::KissOfDeath(kod_code) => {
                    assert_eq!(kod_code, expected_variant);
                    assert_eq!(kod_code.as_str(), code_str);
                }
                _ => unreachable!("Unexpected error code"),
            }
        });

        let _ = executor.spawn(&mut task, &mut handler);

        executor.run();
    }
}

#[test]
fn test_kiss_of_death_experimental() {
    // X-prefixed codes should return Experimental variant
    let code = *b"XABC";
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let data = {
        const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];

        data[0] = 0x24; // LI=0, VN=4, mode=4
        data[12..16].copy_from_slice(&code);
        data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data[40..48].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_read_result(Ok((data.len(), dest)));

    let context = NtpContext::new(MockTimestampGen);
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let mut handler: miniloop::task::Handle<()> = miniloop::task::Handle::default();
    let mut task = miniloop::task::Task::new("test", async {
        let result = sntp_send_request(dest, &socket, context).await;
        assert!(result.is_ok());
        let result = sntp_process_response(dest, &socket, context, result.unwrap()).await;

        match result.unwrap_err() {
            Error::KissOfDeath(KissOfDeathCode::Experimental(inner)) => {
                assert_eq!(inner, code);
                assert_eq!(KissOfDeathCode::Experimental(code).as_str(), "XABC");
            }
            _ => unreachable!("Expected Experimental KoD"),
        }
    });

    let _ = executor.spawn(&mut task, &mut handler);
    executor.run();
}

#[test]
fn test_kiss_of_death_unknown() {
    // Unknown non-X codes should also return Experimental variant
    let code = *b"ZZZZ";
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let data = {
        const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];

        data[0] = 0x24; // LI=0, VN=4, mode=4
        data[12..16].copy_from_slice(&code);
        data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data[40..48].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_read_result(Ok((data.len(), dest)));

    let context = NtpContext::new(MockTimestampGen);
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let mut handler: miniloop::task::Handle<()> = miniloop::task::Handle::default();
    let mut task = miniloop::task::Task::new("test", async {
        let result = sntp_send_request(dest, &socket, context).await;
        assert!(result.is_ok());
        let result = sntp_process_response(dest, &socket, context, result.unwrap()).await;

        match result.unwrap_err() {
            Error::KissOfDeath(KissOfDeathCode::Experimental(inner)) => {
                assert_eq!(inner, code);
                assert_eq!(KissOfDeathCode::Experimental(code).as_str(), "ZZZZ");
            }
            _ => unreachable!("Expected Experimental KoD"),
        }
    });

    let _ = executor.spawn(&mut task, &mut handler);
    executor.run();
}

#[test]
fn test_process_incorrect_origin_timestamp() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    let data = {
        let mut data = [0u8; 48];

        data[0] = 4 | (4 << 3);
        data[1] = 1;
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::IncorrectOriginTimestamp)));
}

#[test]
fn test_unsynchronized_clock() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    let data = {
        const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];

        // LI=3, VN=4, mode=4 => 0b11_100_100 = 0xE4
        data[0] = 0xE4;
        data[1] = 1;
        data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::UnsynchronizedClock)));
}

#[test]
fn test_invalid_timestamp() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    // Packet with zero transmit timestamp
    let data = {
        const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];

        data[0] = 0x24; // LI=0, VN=4, mode=4
        data[1] = 1; // stratum 1
        data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        // tx_timestamp stays as 0 (default)
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::InvalidTimestamp)));
}

#[test]
fn test_invalid_root_distance() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    // Packet with root_delay/2 + root_dispersion >= MAXDISP
    let data = {
        const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        let mut data = [0u8; 48];

        data[0] = 0x24; // LI=0, VN=4, mode=4
        data[1] = 1; // stratum 1
        // root_delay = 0x0020_0000 (32 seconds in NTP short format)
        data[4..8].copy_from_slice(&(0x0020_0000u32).to_be_bytes());
        // root_dispersion = 0 (so root_delay/2 + root_disp = 0x0010_0000 = MAXDISP => should be rejected)
        data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data[40..48].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::ExcessiveRootDistance)));
}

#[test]
fn test_ref_timestamp_newer_than_tx_timestamp() {
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    // Packet with ref_timestamp > tx_timestamp
    let data = {
        const TX_TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
        const REF_TIMESTAMP: u64 = 9_487_534_653_230_284_801u64; // newer than tx
        let mut data = [0u8; 48];

        data[0] = 0x24; // LI=0, VN=4, mode=4
        data[1] = 1; // stratum 1
        data[16..24].copy_from_slice(REF_TIMESTAMP.to_be_bytes().as_ref());
        data[24..32].copy_from_slice(TX_TIMESTAMP.to_be_bytes().as_ref());
        data[40..48].copy_from_slice(TX_TIMESTAMP.to_be_bytes().as_ref());
        data
    };

    let mut socket = MockUdpSocket::new(dest, data);
    socket.update_write_result(Ok(48));
    socket.update_read_result(Ok((48, dest)));
    let mut executor: miniloop::executor::Executor<1> = miniloop::executor::Executor::new();
    let result = executor.block_on(async {
        let resp = sntp_send_request(dest, &socket, context).await;

        sntp_process_response(dest, &socket, context, resp.unwrap()).await
    });

    assert!(result.is_err());
    assert!(matches!(result, Err(Error::BackwardReferenceTimestamp)));
}
