//! Tokio async runtime UDP socket adapter for the [`sntpc`] SNTP client library.
//!
//! This crate provides a wrapper around [`tokio::net::UdpSocket`] that implements
//! the [`NtpUdpSocket`] trait, enabling asynchronous SNTP requests using the Tokio runtime.
//!
//! # Design Rationale
//!
//! The network adapters are separated into their own crates to:
//! - Enable independent versioning (updating Tokio doesn't require updating `sntpc` core)
//! - Allow version flexibility (works with any Tokio 1.x version)
//! - Keep the core SNTP protocol logic independent of async runtimes
//! - Simplify future compatibility (only this adapter needs updating for Tokio 2.x)
//!
//! # Example
//!
//! ```ignore
//! use sntpc::{get_time, NtpContext, StdTimestampGen};
//! use sntpc_net_tokio::UdpSocketWrapper;
//! use tokio::net::UdpSocket;
//!
//! #[tokio::main]
//! async fn main() {
//!     let socket = UdpSocket::bind("0.0.0.0:0")
//!         .await
//!         .expect("Failed to bind socket");
//!     let socket = UdpSocketWrapper::from(socket);
//!     let context = NtpContext::new(StdTimestampGen::default());
//!
//!     let result = get_time("pool.ntp.org:123".parse().unwrap(), &socket, context).await;
//!     match result {
//!         Ok(time) => println!("Received time: {}.{}", time.sec(), time.sec_fraction()),
//!         Err(e) => eprintln!("Failed to get time: {:?}", e),
//!     }
//! }
//! ```
//!
//! For more examples, see the [repository examples](https://github.com/vpetrigo/sntpc/tree/master/examples/tokio).
#![no_std]

use sntpc::{Error, NtpUdpSocket, Result};
use tokio::net::UdpSocket;

use core::net::SocketAddr;

/// A wrapper around [`tokio::net::UdpSocket`] that implements [`NtpUdpSocket`].
///
/// This type allows Tokio UDP sockets to be used with the `sntpc` library for making
/// asynchronous SNTP requests. It integrates seamlessly with Tokio's async runtime
/// and provides non-blocking I/O operations.
///
/// # Example
///
/// ```no_run
/// use sntpc_net_tokio::UdpSocketWrapper;
/// use tokio::net::UdpSocket;
///
/// # async fn example() {
/// let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind socket");
/// let wrapper = UdpSocketWrapper::new(socket);
/// // Use wrapper with sntpc async functions
/// # }
/// ```
pub struct UdpSocketWrapper {
    socket: UdpSocket,
}

impl UdpSocketWrapper {
    /// Creates a new `UdpSocketWrapper` from a [`tokio::net::UdpSocket`].
    ///
    /// # Arguments
    ///
    /// * `socket` - A Tokio UDP socket to wrap
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sntpc_net_tokio::UdpSocketWrapper;
    /// use tokio::net::UdpSocket;
    ///
    /// # async fn example() {
    /// let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind");
    /// let wrapper = UdpSocketWrapper::new(socket);
    /// # }
    /// ```
    #[must_use]
    pub fn new(socket: UdpSocket) -> Self {
        Self { socket }
    }
}

impl From<UdpSocket> for UdpSocketWrapper {
    /// Converts a [`tokio::net::UdpSocket`] into a `UdpSocketWrapper`.
    ///
    /// This provides a convenient way to create a wrapper using `.into()` or `from()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use sntpc_net_tokio::UdpSocketWrapper;
    /// use tokio::net::UdpSocket;
    ///
    /// # async fn example() {
    /// let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind");
    /// let wrapper: UdpSocketWrapper = socket.into();
    /// # }
    /// ```
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
