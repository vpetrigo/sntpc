//! Embassy async runtime UDP socket adapter for the [`sntpc`] SNTP client library.
//!
//! This crate provides a wrapper around [`embassy_net::udp::UdpSocket`] that implements
//! the [`NtpUdpSocket`] trait, enabling asynchronous SNTP requests in embedded systems
//! using the Embassy async runtime.
//!
//! # Design Rationale
//!
//! The network adapters are separated into their own crates to:
//! - Enable independent versioning (updating Embassy doesn't require updating `sntpc` core)
//! - Allow version flexibility (works with embassy-net 0.8.x)
//! - Maintain `no_std` compatibility for embedded systems
//!
//! # Features
//!
//! - `ipv6`: Enables IPv6 protocol support (propagates to `embassy-net`)
//! - `log`: Enables logging support via the `log` crate
//! - `defmt`: Enables logging support via the `defmt` crate for embedded systems
//!
//! **Note**: The `log` and `defmt` features are mutually exclusive. If both are enabled,
//! `defmt` takes priority.
//!
//! # Example
//!
//! ```ignore
//! use sntpc::{get_time, NtpContext};
//! use sntpc_net_embassy::UdpSocketWrapper;
//! use embassy_net::udp::UdpSocket;
//!
//! // Within an Embassy async context
//! let socket = UdpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
//! socket.bind(local_port).unwrap();
//! let socket = UdpSocketWrapper::from(socket);
//!
//! let result = get_time(server_addr, &socket, ntp_context).await;
//! match result {
//!     Ok(time) => defmt::info!("Received time: {}.{}", time.sec(), time.sec_fraction()),
//!     Err(e) => defmt::error!("Failed to get time: {:?}", e),
//! }
//! ```
//!
//! For more examples, see the [repository examples](https://github.com/vpetrigo/sntpc/tree/master/examples/embassy-net).
#![no_std]

/// Logging module that conditionally uses either `defmt` or `log` based on feature flags.
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

/// A wrapper around [`embassy_net::udp::UdpSocket`] that implements [`NtpUdpSocket`].
///
/// This type allows Embassy UDP sockets to be used with the `sntpc` library for making
/// asynchronous SNTP requests in embedded systems. It handles address conversion between
/// standard library types and Embassy's network types.
///
/// The wrapper has a lifetime parameter that matches the underlying Embassy socket's
/// lifetime, typically tied to the network stack and buffer lifetimes.
///
/// # Example
///
/// ```ignore
/// use sntpc_net_embassy::UdpSocketWrapper;
/// use embassy_net::udp::UdpSocket;
///
/// let socket = UdpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
/// socket.bind(8123).unwrap();
/// let wrapper = UdpSocketWrapper::new(socket);
/// // Use wrapper with sntpc async functions
/// ```
pub struct UdpSocketWrapper<'a> {
    socket: UdpSocket<'a>,
}

impl<'a> UdpSocketWrapper<'a> {
    /// Creates a new `UdpSocketWrapper` from an [`embassy_net::udp::UdpSocket`].
    ///
    /// # Arguments
    ///
    /// * `socket` - An Embassy UDP socket to wrap
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sntpc_net_embassy::UdpSocketWrapper;
    /// use embassy_net::udp::UdpSocket;
    ///
    /// let socket = UdpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    /// let wrapper = UdpSocketWrapper::new(socket);
    /// ```
    #[must_use]
    pub fn new(socket: UdpSocket<'a>) -> Self {
        Self { socket }
    }
}

impl<'a> From<UdpSocket<'a>> for UdpSocketWrapper<'a> {
    /// Converts an [`embassy_net::udp::UdpSocket`] into a `UdpSocketWrapper`.
    ///
    /// This provides a convenient way to create a wrapper using `.into()` or `from()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sntpc_net_embassy::UdpSocketWrapper;
    /// use embassy_net::udp::UdpSocket;
    ///
    /// let socket = UdpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    /// let wrapper: UdpSocketWrapper = socket.into();
    /// ```
    fn from(value: UdpSocket<'a>) -> Self {
        UdpSocketWrapper::new(value)
    }
}

/// Converts a standard [`SocketAddr`] to an Embassy [`IpEndpoint`].
///
/// This helper function handles the conversion between standard library network
/// types and Embassy's network types. IPv6 addresses are only supported when
/// the `ipv6` feature is enabled.
///
/// # Panics
///
/// Panics if an IPv6 address is provided without the `ipv6` feature enabled.
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

/// Converts an Embassy [`IpEndpoint`] to a standard [`SocketAddr`].
///
/// This helper function handles the conversion from Embassy's network types
/// back to standard library network types.
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
