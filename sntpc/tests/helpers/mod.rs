use sntpc::{NtpTimestampGenerator, NtpUdpSocket};
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
    write_result: sntpc::Result<usize>,
    read: [u8; 48],
    read_result: sntpc::Result<(usize, SocketAddr)>,
}

impl MockUdpSocket {
    #[must_use]
    pub fn new(dest_addr: SocketAddr, data: [u8; 48]) -> Self {
        Self {
            write_result: Ok(48),
            read: data,
            read_result: Ok((data.len(), dest_addr)),
        }
    }

    pub fn update_write_result(&mut self, value: sntpc::Result<usize>) {
        self.write_result = value;
    }

    pub fn update_read_result(&mut self, value: sntpc::Result<(usize, SocketAddr)>) {
        self.read_result = value;
    }
}

impl NtpUdpSocket for MockUdpSocket {
    async fn send_to(&self, _buf: &[u8], _addr: SocketAddr) -> sntpc::Result<usize> {
        self.write_result
    }

    async fn recv_from(&self, buf: &mut [u8]) -> sntpc::Result<(usize, SocketAddr)> {
        buf.copy_from_slice(&self.read);

        self.read_result
    }
}
