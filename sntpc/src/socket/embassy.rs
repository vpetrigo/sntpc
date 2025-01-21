#[cfg(any(feature = "log", feature = "defmt"))]
use crate::log::error;
use crate::{net::SocketAddr, Error, NtpUdpSocket, Result};
use embassy_net::{udp::UdpSocket, IpAddress, IpEndpoint};

use core::net::IpAddr;

impl NtpUdpSocket for UdpSocket<'_> {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        // Currently smoltcp still has its own address enum
        let endpoint = IpEndpoint::new(
            match addr.ip() {
                IpAddr::V4(addr) => IpAddress::Ipv4(addr),
                #[cfg(feature = "embassy-socket-ipv6")]
                IpAddr::V6(addr) => IpAddress::Ipv6(addr),
                #[allow(unreachable_patterns)]
                _ => unreachable!(),
            },
            addr.port(),
        );

        match UdpSocket::send_to(self, buf, endpoint).await {
            Ok(()) => Ok(buf.len()),
            Err(e) => {
                #[cfg(any(feature = "log", feature = "defmt"))]
                error!("Error while sending to {}: {:?}", endpoint, e);
                Err(Error::Network)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let to_addr = |ep: IpEndpoint| {
            SocketAddr::new(
                match ep.addr {
                    IpAddress::Ipv4(val) => IpAddr::V4(val),
                    #[cfg(feature = "embassy-socket-ipv6")]
                    IpAddress::Ipv6(val) => IpAddr::V6(val),
                    #[allow(unreachable_patterns)]
                    _ => unreachable!(),
                },
                ep.port,
            )
        };

        match UdpSocket::recv_from(self, buf).await {
            Ok((len, ep)) => Ok((len, to_addr(ep.endpoint))),
            Err(e) => {
                #[cfg(any(feature = "log", feature = "defmt"))]
                error!("Error receiving {:?}", e);
                Err(Error::Network)
            }
        }
    }
}
