use sntpc::{Error, NtpUdpSocket, Result};
use tokio::net::UdpSocket;

use core::net::SocketAddr;

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
        self.socket.send_to(buf, addr).await.map_err(|_| Error::Network)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf).await.map_err(|_| Error::Network)
    }
}
