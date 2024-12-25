use crate::{net::SocketAddr, Error, NtpUdpSocket, Result};

use core::net::IpAddr;

impl NtpUdpSocket for &embassy_net::udp::UdpSocket<'_> {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        // Currently smoltcp still has its own address enum
        let endpoint = embassy_net::IpEndpoint::new(
            match addr.ip() {
                IpAddr::V4(addr) => embassy_net::IpAddress::Ipv4(addr),
                IpAddr::V6(addr) => embassy_net::IpAddress::Ipv6(addr),
            },
            addr.port(),
        );

        match embassy_net::udp::UdpSocket::send_to(self, buf, endpoint).await {
            Ok(()) => Ok(buf.len()),
            Err(e) => {
                #[cfg(feature = "log")]
                log::error!("Error while sending to {endpoint}: {e:?}");
                Err(Error::Network)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let to_addr = |ep: embassy_net::IpEndpoint| {
            SocketAddr::new(
                match ep.addr {
                    embassy_net::IpAddress::Ipv4(val) => IpAddr::V4(val),
                    embassy_net::IpAddress::Ipv6(val) => IpAddr::V6(val),
                },
                ep.port,
            )
        };

        match embassy_net::udp::UdpSocket::recv_from(self, buf).await {
            Ok((len, ep)) => Ok((len, to_addr(ep.endpoint))),
            Err(e) => {
                #[cfg(feature = "log")]
                log::error!("Error receiving {e:?}");
                Err(Error::Network)
            }
        }
    }
}
