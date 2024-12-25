use crate::{net::SocketAddr, Error, NtpUdpSocket};

use std::net::UdpSocket;

impl NtpUdpSocket for UdpSocket {
    async fn send_to(
        &self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> crate::Result<usize> {
        match self.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    async fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> crate::Result<(usize, SocketAddr)> {
        match self.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
