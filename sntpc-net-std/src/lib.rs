//! Standard library UDP socket adapter for the [`sntpc`] SNTP client library.
//!
//! This crate provides a thin wrapper around [`std::net::UdpSocket`] that implements
//! the [`NtpUdpSocket`] trait, allowing it to be used with the `sntpc` library for
//! synchronous SNTP requests.
//!
//! # Design Rationale
//!
//! The network adapters are separated into their own crates to:
//! - Allow independent versioning of network implementations
//! - Minimize dependencies (only `std` and `sntpc` core required)
//! - Enable users to choose their preferred network stack
//!
//! # Example
//!
//! ```ignore
//! use sntpc::{sync::get_time, NtpContext, StdTimestampGen};
//! use sntpc_net_std::UdpSocketWrapper;
//! use std::net::UdpSocket;
//!
//! let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to create UDP socket");
//! socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))
//!     .expect("Unable to set read timeout");
//! let socket = UdpSocketWrapper::new(socket);
//! let context = NtpContext::new(StdTimestampGen::default());
//!
//! let result = get_time("pool.ntp.org:123".parse().unwrap(), &socket, context);
//! match result {
//!     Ok(time) => println!("Received time: {}.{}", time.sec(), time.sec_fraction()),
//!     Err(e) => eprintln!("Failed to get time: {:?}", e),
//! }
//! ```
//!
//! For more examples, see the [repository examples](https://github.com/vpetrigo/sntpc/tree/master/examples).

use sntpc::{Error, NtpUdpSocket, Result};

use std::net::{SocketAddr, UdpSocket};

/// A wrapper around [`std::net::UdpSocket`] that implements [`NtpUdpSocket`].
///
/// This type allows standard library UDP sockets to be used with the `sntpc` library
/// for making SNTP requests. It provides a simple synchronous interface suitable for
/// blocking I/O operations.
///
/// # Example
///
/// ```no_run
/// use sntpc_net_std::UdpSocketWrapper;
/// use std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
/// let wrapper = UdpSocketWrapper::new(socket);
/// // Use wrapper with sntpc functions
/// ```
pub struct UdpSocketWrapper {
    socket: UdpSocket,
}

impl UdpSocketWrapper {
    /// Creates a new `UdpSocketWrapper` from a [`std::net::UdpSocket`].
    ///
    /// # Arguments
    ///
    /// * `socket` - A UDP socket to wrap
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sntpc_net_std::UdpSocketWrapper;
    /// use std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");
    /// let wrapper = UdpSocketWrapper::new(socket);
    /// ```
    #[must_use]
    pub fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }
}

impl From<UdpSocket> for UdpSocketWrapper {
    /// Converts a [`std::net::UdpSocket`] into a `UdpSocketWrapper`.
    ///
    /// This provides a convenient way to create a wrapper using `.into()` or `from()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sntpc_net_std::UdpSocketWrapper;
    /// use std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");
    /// let wrapper: UdpSocketWrapper = socket.into();
    /// ```
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
