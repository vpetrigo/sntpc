use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket, sntp_process_response, sntp_send_request};
use std::net::SocketAddr;

#[derive(Default, Copy, Clone)]
pub struct MockTimestampGen;

impl NtpTimestampGenerator for MockTimestampGen {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        0u64
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        0u32
    }
}

pub struct MockUdpSocket {
    dest_addr: SocketAddr,
    to_write_result: sntpc::Result<usize>,
    to_read: [u8; 48],
    to_read_result: sntpc::Result<(usize, SocketAddr)>,
}

impl MockUdpSocket {
    #[must_use]
    pub fn new(dest_addr: SocketAddr, data: [u8; 48]) -> Self {
        Self {
            dest_addr,
            to_write_result: Ok(48),
            to_read: data,
            to_read_result: Ok((data.len(), dest_addr)),
        }
    }

    pub fn update_write_result(&mut self, value: sntpc::Result<usize>) {
        self.to_write_result = value;
    }

    pub fn update_read_result(&mut self, value: sntpc::Result<(usize, SocketAddr)>) {
        self.to_read_result = value;
    }
}

impl NtpUdpSocket for MockUdpSocket {
    async fn send_to(&self, _buf: &[u8], _addr: SocketAddr) -> sntpc::Result<usize> {
        self.to_write_result
    }

    async fn recv_from(&self, buf: &mut [u8]) -> sntpc::Result<(usize, SocketAddr)> {
        buf.copy_from_slice(&self.to_read);

        self.to_read_result
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
