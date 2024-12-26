use crate::{Error, NtpUdpSocket, Result};
use tokio::net::UdpSocket;

use core::net::SocketAddr;

impl NtpUdpSocket for UdpSocket {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        self.send_to(buf, addr).await.map_err(|_| Error::Network)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.recv_from(buf).await.map_err(|_| Error::Network)
    }
}
