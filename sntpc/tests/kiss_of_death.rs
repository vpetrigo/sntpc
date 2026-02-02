use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket, sntp_process_response, sntp_send_request};

use core::net::SocketAddr;

#[derive(Copy, Clone)]
struct MockTimestampGen;

impl MockTimestampGen {
    fn new() -> Self {
        Self
    }
}

impl NtpTimestampGenerator for MockTimestampGen {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        0u64
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        0u32
    }
}

struct MockUdpSocket {
    dest_addr: SocketAddr,
    to_write: [u8; 48],
}

impl MockUdpSocket {
    fn new(dest_addr: SocketAddr, to_write: [u8; 48]) -> Self {
        Self { dest_addr, to_write }
    }
}

impl NtpUdpSocket for MockUdpSocket {
    async fn send_to(&self, buf: &[u8], _addr: SocketAddr) -> sntpc::Result<usize> {
        Ok(buf.len())
    }

    async fn recv_from(&self, buf: &mut [u8]) -> sntpc::Result<(usize, SocketAddr)> {
        buf.copy_from_slice(&self.to_write);
        Ok((48, self.dest_addr))
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

        let socket = MockUdpSocket::new(dest, data);
        let context = NtpContext::new(MockTimestampGen::new());
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

        // let ref_id_bytes = code.as_bytes();
        // let ref_id = u32::from_be_bytes(ref_id_bytes.try_into().unwrap_or([0; 4]));
        // let response = NtpPacket {
        //     li_vn_mode: 0xC4,
        //     stratum: 0,
        //     poll: 0,
        //     precision: 0,
        //     root_delay: 0,
        //     root_dispersion: 0,
        //     ref_id,
        //     ref_timestamp: 0,
        //     origin_timestamp: 0,
        //     recv_timestamp: 0,
        //     tx_timestamp: 0,
        // };
        // let raw: RawNtpPacket = (&response).into();
        // let result = process_response(req_resp, raw, 0);
        //
        // assert!(result.is_err());
        //
        // match result.unwrap_err() {
        //     Error::KissOfDeath(kod_code) => assert_eq!(kod_code.as_str(), code),
        //     _ => unreachable!("Unexpected error code"),
        // }
    }
}
