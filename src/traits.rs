use crate::{Error, NtpUdpSocket};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

impl NtpUdpSocket for UdpSocket {
    fn send_to<T: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize, Error> {
        match self.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        match self.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
