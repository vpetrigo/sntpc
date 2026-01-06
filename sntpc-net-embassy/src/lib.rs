#[cfg(any(feature = "log", feature = "defmt"))]
mod log {
    use cfg_if::cfg_if;

    cfg_if! {
        if #[cfg(feature = "defmt")] {
            pub(crate) use defmt::error;
        } else if #[cfg(feature = "log")] {
            pub(crate) use log::error;
        }
    }
}

#[cfg(any(feature = "log", feature = "defmt"))]
use crate::log::error;

use embassy_net::{IpAddress, IpEndpoint, udp::UdpSocket};
use sntpc::{Error, NtpUdpSocket, Result};

use core::net::{IpAddr, SocketAddr};

pub struct UdpSocketWrapper<'a> {
    socket: UdpSocket<'a>,
}

impl<'a> UdpSocketWrapper<'a> {
    #[must_use]
    pub fn new(socket: UdpSocket<'a>) -> Self {
        Self { socket }
    }
}

impl<'a> From<UdpSocket<'a>> for UdpSocketWrapper<'a> {
    fn from(value: UdpSocket<'a>) -> Self {
        UdpSocketWrapper::new(value)
    }
}

fn to_endpoint(addr: SocketAddr) -> IpEndpoint {
    // Currently smoltcp/embassy-net still has its own address enum
    IpEndpoint::new(
        match addr.ip() {
            IpAddr::V4(addr) => IpAddress::Ipv4(addr),
            #[cfg(feature = "ipv6")]
            IpAddr::V6(addr) => IpAddress::Ipv6(addr),
            #[cfg(not(feature = "ipv6"))]
            _ => unreachable!(),
        },
        addr.port(),
    )
}

fn from_endpoint(ep: IpEndpoint) -> SocketAddr {
    SocketAddr::new(
        match ep.addr {
            IpAddress::Ipv4(val) => IpAddr::V4(val),
            #[cfg(feature = "ipv6")]
            IpAddress::Ipv6(val) => IpAddr::V6(val),
        },
        ep.port,
    )
}

impl NtpUdpSocket for UdpSocketWrapper<'_> {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        let endpoint = to_endpoint(addr);

        match self.socket.send_to(buf, endpoint).await {
            Ok(()) => Ok(buf.len()),
            #[allow(unused_variables)]
            Err(e) => {
                #[cfg(any(feature = "log", feature = "defmt"))]
                error!("Error while sending to {}: {:?}", endpoint, e);
                Err(Error::Network)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.socket.recv_from(buf).await {
            Ok((len, ep)) => Ok((len, from_endpoint(ep.endpoint))),
            #[allow(unused_variables)]
            Err(e) => {
                #[cfg(any(feature = "log", feature = "defmt"))]
                error!("Error receiving {:?}", e);
                Err(Error::Network)
            }
        }
    }
}
