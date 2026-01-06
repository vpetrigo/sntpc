use sntpc::{Error, NtpUdpSocket, Result};

use std::net::{SocketAddr, UdpSocket};

pub struct UdpSocketWrapper {
    socket: UdpSocket,
}

impl UdpSocketWrapper {
    #[must_use]
    pub fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }
}

impl From<UdpSocket> for UdpSocketWrapper {
    fn from(socket: UdpSocket) -> Self {
        UdpSocketWrapper::new(socket)
    }
}

impl NtpUdpSocket for UdpSocketWrapper {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        match self.socket.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.socket.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
