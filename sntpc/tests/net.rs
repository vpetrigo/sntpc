mod helpers;

use helpers::*;
use sntpc::{Error, NtpContext, sntp_process_response, sntp_send_request};

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
fn test_process_incorrect_mode() {
    const PROPER_SNTP_MODE1: u8 = 4;
    const PROPER_SNTP_MODE2: u8 = 5;
    let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
    let context = NtpContext::new(MockTimestampGen);

    for i in (0..=0b111u8).filter(|&i| (i != PROPER_SNTP_MODE1) && (i != PROPER_SNTP_MODE2)) {
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
    let defined_codes = [
        "ACST", "AUTH", "AUTO", "BCST", "CRYP", "DENY", "DROP", "RSTR", "INIT", "MCST", "NKEY", "RATE", "RMOT", "STEP",
    ];

    for code in defined_codes {
        let dest: SocketAddr = "127.0.0.1:123".parse().unwrap();
        let data = {
            const TIMESTAMP: u64 = 9_487_534_653_230_284_800u64;
            let mut data = [0u8; 48];

            data[0] = 0xE4;
            data[12..16].copy_from_slice(code.as_bytes());
            data[24..32].copy_from_slice(TIMESTAMP.to_be_bytes().as_ref());
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
                Error::KissOfDeath(kod_code) => assert_eq!(kod_code.as_str(), code),
                _ => unreachable!("Unexpected error code"),
            }
        });

        let _ = executor.spawn(&mut task, &mut handler);

        executor.run();
    }
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
