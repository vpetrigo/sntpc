//! Rust SNTP client
//!
//! # Overview
//!
//! This crate provides a method for sending requests to NTP servers
//! and process responses, extracting received timestamp. Supported SNTP protocol
//! versions:
//! - [SNTPv4](https://datatracker.ietf.org/doc/html/rfc4330)
//!
//! # Usage
//!
//! Put this in your `Cargo.toml`:
//! ```cargo
//! [dependencies]
//! sntpc = "0.5"
//! ```
//!
//! ## Features
//!
//! `sntpc` supports several features:
//! - `std`: includes functionality that depends on the standard library
//! - `sync`: enables synchronous interface
//! - `utils`: includes functionality that mostly OS specific and allows system time sync
//! - `log`: enables library debug output during execution
//! - `std-socket`: add `NtpUdpSocket` trait implementation for `std::net::UdpSocket`
//! - `embassy-socket`: add `NtpUdpSocket` trait implementation for `embassy_net::udp::UdpSocket`
//! - `tokio-socket`: add `NtpUdpSocket` trait implementation for `tokio::net::UdpSocket`
//!
//! <div class="warning">
//!
//! **Warning**: `utils` feature is not stable and may change in the future.
//! </div>
//!
//! # Details
//!
//! There are multiple approaches how the library can be used:
//! - under environments where a networking stuff is hidden in system/RTOS kernel, [`get_time`] can
//!   be used since it encapsulates network I/O
//! - under environments where TCP/IP stack requires to call some helper functions like `poll`,
//!   `wait`, etc. and/or there are no options to perform I/O operations within a single call,
//!   [`sntp_send_request`] and [`sntp_process_response`] can be used
//!
//! As `sntpc` supports `no_std` environment as well, it was
//! decided to provide a set of traits to implement for a network object (`UdpSocket`)
//! and timestamp generator:
//! - [`NtpUdpSocket`] trait should be implemented for `UdpSocket`-like objects for the
//!   library to be able to send and receive data from NTP servers
//! - [`NtpTimestampGenerator`] trait should be implemented for timestamp generator objects to
//!   provide the library with system related timestamps
//!
//! ## Logging support
//!
//! Library debug logs can be enabled in executables by enabling `log` feature. Server
//! addresses, response payload will be printed.
//!
//! # Example
//!
//! ```rust
//! use sntpc::{get_time, NtpContext, NtpUdpSocket, NtpTimestampGenerator, Result};
//! # use miniloop::executor::Executor;
//! use std::net::{SocketAddr, ToSocketAddrs};
//! use core::net::{IpAddr, Ipv4Addr};
//! # #[cfg(feature="std")]
//! use std::net::UdpSocket;
//!
//! #[derive(Copy, Clone)]
//! struct Timestamp;
//! # #[cfg(not(feature="std"))]
//! # #[derive(Debug, Clone)]
//! # struct UdpSocket;
//!
//! impl NtpTimestampGenerator for Timestamp {
//!     fn init(&mut self) {
//!         // ...
//!     }
//!     fn timestamp_sec(&self) -> u64 {
//!         0u64
//!     }
//!     fn timestamp_subsec_micros(&self) -> u32 {
//!         0u32
//!     }
//! }
//!
//! impl Default for Timestamp {
//!     fn default() -> Self {
//!         Self {}
//!     }
//! }
//! # #[cfg(not(feature = "std"))]
//! # impl UdpSocket {
//! #     fn bind(addr: &str) -> Result<Self> {
//! #         Ok(UdpSocket{})
//! #     }
//! #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
//! #        Ok(0usize)
//! #     }
//! #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
//! #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
//! #     }
//! # }
//! # #[cfg(not(feature="std"))]
//! # impl NtpUdpSocket for UdpSocket {
//! #     async fn send_to(
//! #         &self,
//! #         buf: &[u8],
//! #         addr: SocketAddr,
//! #     ) -> Result<usize> {
//! #         match self.send_to(buf, addr) {
//! #             Ok(usize) => Ok(usize),
//! #             Err(_) => Err(sntpc::Error::Network),
//! #         }
//! #     }
//! #
//! #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
//! #         match self.recv_from(buf) {
//! #             Ok((size, addr)) => Ok((size, addr)),
//! #             Err(_) => Err(sntpc::Error::Network),
//! #         }
//! #     }
//! # }
//!
//! fn main() {
//!     let socket =
//!         UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
//!     let context = NtpContext::new(Timestamp::default());
//!     # let mut executor = Executor::new();
//!     let server_addr: SocketAddr = "time.google.com:123"
//!         .to_socket_addrs()
//!         .expect("Unable to resolve host")
//!         .next()
//!         .unwrap();
//!
//!     match executor
//!         .block_on(async { get_time(server_addr, &socket, context).await })
//!     {
//!         Ok(response_result) => {
//!             println!("Response processed: {response_result:?}")
//!         }
//!         Err(err) => eprintln!("Error: {err:?}"),
//!     }
//! }
//! ```
//!
//! For more complex example with a custom timestamp generator and UDP socket implementation, see
//! [`examples/smoltcp-request`](examples/smoltcp-request).
//!
//! For usage SNTP-client in an asynchronous environment, see [`examples/tokio`](examples/tokio)
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "utils")]
pub mod utils;

mod socket;
mod types;

pub use crate::types::*;

#[cfg(feature = "log")]
use core::str;

/// Network types used by the `sntpc` crate
pub mod net {
    pub use core::net::SocketAddr;

    #[cfg(feature = "std")]
    pub use std::net::UdpSocket;
}

#[cfg(feature = "log")]
use log::debug;

/// Retrieves the current time from an NTP server.
///
/// This asynchronous function performs the complete SNTP flow:
/// sending a request to the specified NTP server and processing the server's response.
/// It calculates the roundtrip delay, time offset, and other relevant information.
///
/// # Arguments
///
/// * `addr` - The socket address (`SocketAddr`) of the NTP server.
/// * `socket` - A reference to an object implementing the [`NtpUdpSocket`] trait that allows
///    sending/receiving UDP packets.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator that implements
///    the [`NtpTimestampGenerator`] trait. This ensures precise timestamp creation for request and response processing.
///
/// # Returns
///
/// Returns a `Result<NtpResult>`:
/// * `Ok(NtpResult)` - Successfully retrieved time from the server, including:
///   - Roundtrip delay
///   - Time offset
///   - Stratum level
///   - Precision
/// * `Err(Error)` - Encountered an error, such as:
///   - Network communication issues
///   - Incorrect or invalid server response
///
/// # Examples
///
/// ```rust
/// use sntpc::{get_time, NtpContext, NtpUdpSocket, NtpTimestampGenerator, Result};
/// # use miniloop::executor::Executor;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// use core::net::{IpAddr, Ipv4Addr};
/// # #[cfg(feature="std")]
/// use std::net::UdpSocket;
///
/// #[derive(Copy, Clone)]
/// struct Timestamp;
/// # #[cfg(not(feature="std"))]
/// #[derive(Debug, Clone)]
/// struct UdpSocket;
///
/// impl NtpTimestampGenerator for Timestamp {
///     fn init(&mut self) {
///         // ...
///     }
///     fn timestamp_sec(&self) -> u64 {
///         0u64
///     }
///     fn timestamp_subsec_micros(&self) -> u32 {
///         0u32
///     }
/// }
///
/// impl Default for Timestamp {
///     fn default() -> Self {
///         Self {}
///     }
/// }
///
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket{})
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(0usize)
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// # #[cfg(not(feature="std"))]
/// # impl NtpUdpSocket for UdpSocket {
/// #     async fn send_to(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: SocketAddr,
/// #     ) -> Result<usize> {
/// #         match self.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
///
/// fn main() {
///     let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
///     let context = NtpContext::new(Timestamp::default());
///     let server_addr: SocketAddr = "time.google.com:123".to_socket_addrs().expect("Unable to resolve host").next().unwrap();
///     # let mut executor = Executor::new();
///
///     match executor.block_on(async {
///         get_time(server_addr, &socket, context).await
///     })
///     {
///         Ok(response_result) => println!("Response processed: {response_result:?}"),
///         Err(err) => eprintln!("Error: {err:?}"),
///     }
/// }
/// ```
///
/// # Errors
///
/// This function returns an `Err` in any of the following cases:
/// * The SNTP packet could not be sent to the server.
/// * The response payload is invalid or indicates an error.
/// * Mismatch between the expected and actual server addresses.
pub async fn get_time<U, T>(
    addr: net::SocketAddr,
    socket: &U,
    context: NtpContext<T>,
) -> Result<NtpResult>
where
    U: NtpUdpSocket,
    T: NtpTimestampGenerator + Copy,
{
    let result = sntp_send_request(addr, socket, context).await?;

    sntp_process_response(addr, socket, context, result).await
}

/// Sends an SNTP request to an NTP server.
///
/// This function creates an SNTP packet using the given timestamp generator and
/// sends it to the given NTP server via the provided UDP socket.
///
/// # Arguments
///
/// * `dest` - The socket address (`SocketAddr`) of the NTP server.
/// * `socket` - A reference to an object implementing the [`NtpUdpSocket`] trait
///    that is used to send/receive UDP packets.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator
///    that implements the [`NtpTimestampGenerator`] trait to provide a custom mechanism for generating timestamps.
///
/// # Returns
///
/// Returns a `Result<SendRequestResult>`:
/// * `Ok(SendRequestResult)` - If the packet was successfully sent, includes details
///    about the request, such as the originate timestamp.
/// * `Err(Error)` - If there was an error in sending the request, such as a network failure.
///
/// # Examples
///
/// ```rust
/// use sntpc::{sntp_process_response, sntp_send_request, NtpContext, NtpTimestampGenerator, NtpUdpSocket, Result};
/// # use miniloop::executor::Executor;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// use core::net::{IpAddr, Ipv4Addr};
/// # #[cfg(feature="std")]
/// use std::net::UdpSocket;
///
/// #[derive(Copy, Clone)]
/// struct Timestamp;
/// # #[cfg(not(feature="std"))]
/// #[derive(Debug, Clone)]
/// struct UdpSocket;
///
/// impl NtpTimestampGenerator for Timestamp {
///     fn init(&mut self) {
///         // ...
///     }
///     fn timestamp_sec(&self) -> u64 {
///         0u64
///     }
///     fn timestamp_subsec_micros(&self) -> u32 {
///         0u32
///     }
/// }
///
/// impl Default for Timestamp {
///     fn default() -> Self {
///         Self {}
///     }
/// }
///
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket{})
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(0usize)
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// # #[cfg(not(feature="std"))]
/// # impl NtpUdpSocket for UdpSocket {
/// #     async fn send_to(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: SocketAddr,
/// #     ) -> Result<usize> {
/// #         match self.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
///
/// fn main() {
///     let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
///     let context = NtpContext::new(Timestamp::default());
///     let server_addr: SocketAddr = "time.google.com:123".to_socket_addrs().expect("Unable to resolve host").next().unwrap();
///     # let mut executor = Executor::new();
///
///     let request_result = executor.block_on(async {
///            sntp_send_request(server_addr, &socket, context).await
///     });
/// }
/// ```
///
/// # Errors
///
/// Returns `Err` if:
/// * The SNTP packet fails to send to the provided address due to network issues.
/// * The socket behavior does not conform to the expectations of the [`NtpUdpSocket`] trait.
pub async fn sntp_send_request<U, T>(
    dest: net::SocketAddr,
    socket: &U,
    context: NtpContext<T>,
) -> Result<SendRequestResult>
where
    U: NtpUdpSocket,
    T: NtpTimestampGenerator,
{
    #[cfg(feature = "log")]
    debug!("send request - Address: {:?}", dest);
    let request = NtpPacket::new(context.timestamp_gen);

    send_request(dest, &request, socket).await?;
    Ok(SendRequestResult::from(request))
}

/// Processes the response from an NTP server.
///
/// This function validates the SNTP response, ensuring that it comes from the expected server and that
/// the payload size and structure are correct. It then calculates and returns the offset and
/// roundtrip delay based on the time information.
///
/// # Arguments
///
/// * `dest` - The expected socket address (`SocketAddr`) of the NTP server.
/// * `socket` - A reference to an object implementing the [`NtpUdpSocket`] trait
///    used for receiving the response.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator
///    that manages internal time calculations.
/// * `send_req_result` - The result of the previously sent request, containing the originate timestamp
///    of the SNTP request.
///
/// # Returns
///
/// Returns a `Result<NtpResult>`:
/// * `Ok(NtpResult)` - If the response is valid, includes:
///   - Calculated clock offset
///   - Roundtrip delay
///   - Stratum level
///   - Precision level
/// * `Err(Error)` - On failure, for reasons such as:
///   - Mismatched server response address
///   - Invalid packet size or structure
///   - Incorrect mode or incorrect originate timestamp in the response
///
/// # Examples
///
/// ```rust
/// use sntpc::{sntp_process_response, sntp_send_request, NtpContext, NtpTimestampGenerator, NtpUdpSocket, Result};
/// # use miniloop::executor::Executor;
/// use std::net::{SocketAddr, ToSocketAddrs};
/// use core::net::{IpAddr, Ipv4Addr};
/// # #[cfg(feature="std")]
/// use std::net::UdpSocket;
///
/// #[derive(Copy, Clone)]
/// struct Timestamp;
/// # #[cfg(not(feature="std"))]
/// #[derive(Debug, Clone)]
/// struct UdpSocket;
///
/// impl NtpTimestampGenerator for Timestamp {
///     fn init(&mut self) {
///         // ...
///     }
///     fn timestamp_sec(&self) -> u64 {
///         0u64
///     }
///     fn timestamp_subsec_micros(&self) -> u32 {
///         0u32
///     }
/// }
///
/// impl Default for Timestamp {
///     fn default() -> Self {
///         Self {}
///     }
/// }
///
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket{})
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(buf.len())
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// # #[cfg(not(feature="std"))]
/// # impl NtpUdpSocket for UdpSocket {
/// #     async fn send_to(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: SocketAddr,
/// #     ) -> Result<usize> {
/// #         match self.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
///
/// fn main() {
///     let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
///     let context = NtpContext::new(Timestamp::default());
///     let server_addr: SocketAddr = "time.google.com:123".to_socket_addrs().expect("Unable to resolve host").next().unwrap();
///     # let mut executor = Executor::new();
///
///     let request_result = executor.block_on(async {
///            sntp_send_request(server_addr, &socket, context).await.expect("Unable to send request")
///     });
///
///     match executor.block_on(async {
///         sntp_process_response(server_addr, &socket, context, request_result).await
///     })
///     {
///         Ok(response_result) => println!("Response processed: {response_result:?}"),
///         Err(err) => eprintln!("Error: {err:?}"),
///     }
/// }
/// ```
///
/// # Errors
///
/// This function returns an `Err` in any of the following situations:
/// * The source address of the response does not match the server address used for the request.
/// * The size of the response is incorrect or does not match the expected format.
/// * The mode or version in the response is invalid.
pub async fn sntp_process_response<U, T>(
    dest: net::SocketAddr,
    socket: &U,
    mut context: NtpContext<T>,
    send_req_result: SendRequestResult,
) -> Result<NtpResult>
where
    U: NtpUdpSocket,
    T: NtpTimestampGenerator,
{
    let mut response_buf = RawNtpPacket::default();
    let (response, src) = socket.recv_from(response_buf.0.as_mut()).await?;
    context.timestamp_gen.init();
    let recv_timestamp = get_ntp_timestamp(&context.timestamp_gen);
    #[cfg(feature = "log")]
    debug!("Response: {}", response);

    if dest != src {
        return Err(Error::ResponseAddressMismatch);
    }

    if response != size_of::<NtpPacket>() {
        return Err(Error::IncorrectPayload);
    }

    let result =
        process_response(send_req_result, response_buf, recv_timestamp);

    #[cfg(feature = "log")]
    if let Ok(r) = &result {
        debug!("{:?}", r);
    }

    result
}

async fn send_request<U>(
    dest: net::SocketAddr,
    req: &NtpPacket,
    socket: &U,
) -> Result<()>
where
    U: NtpUdpSocket,
{
    let buf = RawNtpPacket::from(req);

    match socket.send_to(&buf.0, dest).await {
        Ok(size) => {
            if size == buf.0.len() {
                Ok(())
            } else {
                Err(Error::Network)
            }
        }
        Err(_) => Err(Error::Network),
    }
}

/// Synchronous interface for the SNTP client
#[cfg(feature = "sync")]
pub mod sync {
    use crate::net;
    use crate::types::{
        NtpContext, NtpResult, NtpTimestampGenerator, NtpUdpSocket, Result,
        SendRequestResult,
    };

    use miniloop::executor::Executor;

    #[cfg(feature = "log")]
    use log::debug;

    /// Send request to a NTP server with the given address and process the response in a single call
    ///
    /// May be useful under an environment with `std` networking implementation, where all
    /// network stuff is hidden within system's kernel. For environment with custom
    /// Uses [`NtpUdpSocket`] and [`NtpTimestampGenerator`] trait bounds to allow generic specification
    /// of objects that can be used with the library
    ///
    /// # Arguments
    ///
    /// - `pool_addrs` - Server's name or IP address with port specification as a string
    /// - `socket` - UDP socket object that will be used during NTP request-response
    ///   communication
    /// - `context` - SNTP client context to provide timestamp generation feature
    ///
    /// # Errors
    ///
    /// Will return `Err` if an SNTP request cannot be sent or SNTP response fails
    pub fn get_time<U, T>(
        addr: net::SocketAddr,
        socket: &U,
        context: NtpContext<T>,
    ) -> Result<NtpResult>
    where
        U: NtpUdpSocket,
        T: NtpTimestampGenerator + Copy,
    {
        let result = sntp_send_request(addr, socket, context)?;
        #[cfg(feature = "log")]
        debug!("{:?}", result);

        sntp_process_response(addr, socket, context, result)
    }

    /// Send an SNTP request to the specified destination synchronously.
    ///
    /// This function is a synchronous wrapper for the asynchronous [`crate::sntp_send_request`].
    /// It uses an executor to block the current thread while waiting for the underlying
    /// asynchronous operation to complete.
    ///
    /// # Arguments
    ///
    /// * `dest` - The destination NTP server's socket address to send the request to.
    /// * `socket` - A reference to an object implementing the [`NtpUdpSocket`] trait to send/receive data.
    /// * `context` - The SNTP client context (implementing [`NtpTimestampGenerator`]) that
    ///    assists in generating timestamps for the request.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the underlying async SNTP request fails for any reason,
    /// such as network failure, invalid server response, or timeout.
    ///
    /// # Examples
    ///
    /// ```
    /// use sntpc::{self, NtpContext, NtpTimestampGenerator, Result};
    /// # use core::future::Future;
    /// # use std::time::Duration;
    /// # use std::str::FromStr;
    /// # #[cfg(feature = "std")]
    /// # use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
    /// # #[derive(Debug)]
    /// # struct UdpSocketWrapper(UdpSocket);
    /// #
    /// # impl sntpc::NtpUdpSocket for UdpSocketWrapper {
    /// #     async fn send_to(
    /// #         &self,
    /// #         buf: &[u8],
    /// #         addr: SocketAddr,
    /// #     ) -> Result<usize> {
    /// #         self.0.send_to(buf, addr).map_err(|_| sntpc::Error::Network)
    /// #     }
    /// #
    /// #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
    /// #         self.0.recv_from(buf).map_err(|_| sntpc::Error::Network)
    /// #     }
    /// # }
    /// # #[derive(Copy, Clone, Default)]
    /// # struct StdTimestampGen {
    /// #     duration: Duration,
    /// # }
    /// #
    /// # impl NtpTimestampGenerator for StdTimestampGen {
    /// #     fn init(&mut self) {
    /// #         self.duration = std::time::SystemTime::now()
    /// #             .duration_since(std::time::SystemTime::UNIX_EPOCH)
    /// #             .unwrap();
    /// #     }
    /// #
    /// #     fn timestamp_sec(&self) -> u64 {
    /// #         self.duration.as_secs()
    /// #     }
    /// #
    /// #     fn timestamp_subsec_micros(&self) -> u32 {
    /// #         self.duration.subsec_micros()
    /// #     }
    /// # }
    /// let socket = UdpSocketWrapper(UdpSocket::bind("0.0.0.0:0").unwrap());
    /// let context = NtpContext::new(StdTimestampGen::default());
    /// // "time.google.com:123" string here used for the sake of simplicity. In the real app
    /// // you would want to fix destination address, since string hostname may resolve to
    /// // different IP addresses
    /// let addr = "time.google.com:123".to_socket_addrs().unwrap().next().unwrap();
    ///
    /// let result = sntpc::sync::sntp_send_request(addr, &socket, context);
    /// match result {
    ///     Ok(response) => println!("Received response: {:?}", response),
    ///     Err(e) => eprintln!("Failed to send request: {:?}", e),
    /// }
    /// ```
    pub fn sntp_send_request<U, T>(
        dest: net::SocketAddr,
        socket: &U,
        context: NtpContext<T>,
    ) -> Result<SendRequestResult>
    where
        U: NtpUdpSocket,
        T: NtpTimestampGenerator + Copy,
    {
        Executor::new()
            .block_on(crate::sntp_send_request(dest, socket, context))
    }

    /// Processes the response from an SNTP server and calculates the NTP offset and round-trip delay.
    ///
    /// This is a synchronous wrapper around the asynchronous SNTP response processing function.
    /// It uses an executor to block the current thread while waiting for the underlying
    /// asynchronous operation to complete.
    ///
    /// # Arguments
    ///
    /// - `dest` - The destination NTP server's socket address from which the response was received.
    /// - `socket` - A reference to an object implementing the [`NtpUdpSocket`] trait used for network communication.
    /// - `context` - The SNTP client context (implementing [`NtpTimestampGenerator`]) responsible for generating and validating timestamps.
    /// - `send_req_result` - The result obtained from sending the SNTP request, including the originate timestamp.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the underlying async SNTP response processing fails for any reason,
    /// such as:
    /// - Incorrect origin timestamp in the response,
    /// - An invalid mode in the response (`SNTP_UNICAST` or `SNTP_BROADCAST`),
    /// - A mismatch between the request and response versions,
    /// - Errors in the response headers (e.g., incorrect stratum, leap indicator),
    /// - Network errors during processing.
    ///
    /// # Examples
    ///
    /// ```
    /// use sntpc::{self, NtpContext, NtpTimestampGenerator, Result, SendRequestResult};
    /// # use core::future::Future;
    /// # use std::time::Duration;
    /// # use std::str::FromStr;
    /// # #[cfg(feature = "std")]
    /// # use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
    /// # #[derive(Debug)]
    /// # struct UdpSocketWrapper(UdpSocket);
    /// #
    /// # impl sntpc::NtpUdpSocket for UdpSocketWrapper {
    /// #     async fn send_to(
    /// #         &self,
    /// #         buf: &[u8],
    /// #         addr: SocketAddr,
    /// #     ) -> Result<usize> {
    /// #         self.0.send_to(buf, addr).map_err(|_| sntpc::Error::Network)
    /// #     }
    /// #
    /// #     async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
    /// #         self.0.recv_from(buf).map_err(|_| sntpc::Error::Network)
    /// #     }
    /// # }
    /// # #[derive(Copy, Clone, Default)]
    /// # struct StdTimestampGen {
    /// #     duration: Duration,
    /// # }
    /// #
    /// # impl NtpTimestampGenerator for StdTimestampGen {
    /// #     fn init(&mut self) {
    /// #         self.duration = std::time::SystemTime::now()
    /// #             .duration_since(std::time::SystemTime::UNIX_EPOCH)
    /// #             .unwrap();
    /// #     }
    /// #
    /// #     fn timestamp_sec(&self) -> u64 {
    /// #         self.duration.as_secs()
    /// #     }
    /// #
    /// #     fn timestamp_subsec_micros(&self) -> u32 {
    /// #         self.duration.subsec_micros()
    /// #     }
    /// # }
    /// let socket = UdpSocketWrapper(UdpSocket::bind("0.0.0.0:0").unwrap());
    /// let context = NtpContext::new(StdTimestampGen::default());
    /// // "time.google.com:123" string here used for the sake of simplicity. In the real app
    /// // you would want to fix destination address, since string hostname may resolve to
    /// // different IP addresses
    /// let addr = "time.google.com:123".to_socket_addrs().unwrap().next().unwrap();
    ///
    /// let send_request_result = sntpc::sync::sntp_send_request(addr, &socket, context).unwrap();
    /// let result = sntpc::sync::sntp_process_response(addr, &socket, context, send_request_result);
    ///
    /// match result {
    ///     Ok(ntp_result) => println!("NTP Result: {:?}", ntp_result),
    ///     Err(e) => eprintln!("Failed to process response: {:?}", e),
    /// }
    /// ```
    pub fn sntp_process_response<U, T>(
        dest: net::SocketAddr,
        socket: &U,
        context: NtpContext<T>,
        send_req_result: SendRequestResult,
    ) -> Result<NtpResult>
    where
        U: NtpUdpSocket,
        T: NtpTimestampGenerator + Copy,
    {
        Executor::new().block_on(crate::sntp_process_response(
            dest,
            socket,
            context,
            send_req_result,
        ))
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn process_response(
    send_req_result: SendRequestResult,
    resp: RawNtpPacket,
    recv_timestamp: u64,
) -> Result<NtpResult> {
    const SNTP_UNICAST: u8 = 4;
    const SNTP_BROADCAST: u8 = 5;
    const LI_MAX_VALUE: u8 = 3;
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);
    #[cfg(feature = "log")]
    debug_ntp_packet(&packet, recv_timestamp);

    if send_req_result.originate_timestamp != packet.origin_timestamp {
        return Err(Error::IncorrectOriginTimestamp);
    }
    // Shift is 0
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let resp_version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let req_version =
        shifter(send_req_result.version, VERSION_MASK, VERSION_SHIFT);

    if mode != SNTP_UNICAST && mode != SNTP_BROADCAST {
        return Err(Error::IncorrectMode);
    }

    if li > LI_MAX_VALUE {
        return Err(Error::IncorrectLeapIndicator);
    }

    if req_version != resp_version {
        return Err(Error::IncorrectResponseVersion);
    }

    if packet.stratum == 0 {
        return Err(Error::IncorrectStratumHeaders);
    }
    // System clock offset:
    // theta = T(B) - T(A) = 1/2 * [(T2-T1) + (T3-T4)]
    // Round-trip delay:
    // delta = T(ABA) = (T4-T1) - (T3-T2).
    // where:
    // - T1 = client's TX timestamp
    // - T2 = server's RX timestamp
    // - T3 = server's TX timestamp
    // - T4 = client's RX timestamp
    let t1 = packet.origin_timestamp;
    let t2 = packet.recv_timestamp;
    let t3 = packet.tx_timestamp;
    let t4 = recv_timestamp;
    let units = Units::Microseconds;
    let roundtrip = roundtrip_calculate(t1, t2, t3, t4, units);
    let offset = offset_calculate(t1, t2, t3, t4, units);
    let timestamp = NtpTimestamp::from(packet.tx_timestamp);

    #[cfg(feature = "log")]
    debug!(
        "Roundtrip delay: {} {}. Offset: {} {}",
        roundtrip, units, offset, units
    );

    Ok(NtpResult::new(
        timestamp.seconds as u32,
        timestamp.seconds_fraction as u32,
        roundtrip,
        offset,
        packet.stratum,
        packet.precision,
    ))
}

fn shifter(val: u8, mask: u8, shift: u8) -> u8 {
    (val & mask) >> shift
}

fn convert_from_network(packet: &mut NtpPacket) {
    fn ntohl<T: NtpNum>(val: &T) -> T::Type {
        val.ntohl()
    }

    packet.root_delay = ntohl(&packet.root_delay);
    packet.root_dispersion = ntohl(&packet.root_dispersion);
    packet.ref_id = ntohl(&packet.ref_id);
    packet.ref_timestamp = ntohl(&packet.ref_timestamp);
    packet.origin_timestamp = ntohl(&packet.origin_timestamp);
    packet.recv_timestamp = ntohl(&packet.recv_timestamp);
    packet.tx_timestamp = ntohl(&packet.tx_timestamp);
}

fn convert_delays(sec: u64, fraction: u64, units: u64) -> u64 {
    sec * units + fraction * units / u64::from(u32::MAX)
}

fn roundtrip_calculate(
    t1: u64,
    t2: u64,
    t3: u64,
    t4: u64,
    units: Units,
) -> u64 {
    let delta = t4.wrapping_sub(t1).saturating_sub(t3.wrapping_sub(t2));
    let delta_sec = (delta & SECONDS_MASK) >> 32;
    let delta_sec_fraction = delta & SECONDS_FRAC_MASK;

    match units {
        Units::Milliseconds => convert_delays(
            delta_sec,
            delta_sec_fraction,
            u64::from(MSEC_IN_SEC),
        ),
        Units::Microseconds => convert_delays(
            delta_sec,
            delta_sec_fraction,
            u64::from(USEC_IN_SEC),
        ),
    }
}

#[allow(clippy::cast_possible_wrap)]
fn offset_calculate(t1: u64, t2: u64, t3: u64, t4: u64, units: Units) -> i64 {
    let theta = (t2.wrapping_sub(t1) as i64 / 2)
        .saturating_add(t3.wrapping_sub(t4) as i64 / 2);
    let theta_sec = (theta.unsigned_abs() & SECONDS_MASK) >> 32;
    let theta_sec_fraction = theta.unsigned_abs() & SECONDS_FRAC_MASK;

    match units {
        Units::Milliseconds => {
            convert_delays(
                theta_sec,
                theta_sec_fraction,
                u64::from(MSEC_IN_SEC),
            ) as i64
                * theta.signum()
        }
        Units::Microseconds => {
            convert_delays(
                theta_sec,
                theta_sec_fraction,
                u64::from(USEC_IN_SEC),
            ) as i64
                * theta.signum()
        }
    }
}

#[cfg(feature = "log")]
fn debug_ntp_packet(packet: &NtpPacket, recv_timestamp: u64) {
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let delimiter_gen = || unsafe { str::from_utf8_unchecked(&[b'='; 64]) };

    debug!("{}", delimiter_gen());
    debug!("| Mode:\t\t{}", mode);
    debug!("| Version:\t{}", version);
    debug!("| Leap:\t\t{}", li);
    debug!("| Stratum:\t{}", packet.stratum);
    debug!("| Poll:\t\t{}", packet.poll);
    debug!("| Precision:\t\t{}", packet.precision);
    debug!("| Root delay:\t\t{}", packet.root_delay);
    debug!("| Root dispersion:\t{}", packet.root_dispersion);
    debug!(
        "| Reference ID:\t\t{}",
        str::from_utf8(&packet.ref_id.to_be_bytes()).unwrap_or("")
    );
    debug!(
        "| Origin timestamp    (client):\t{:>16}",
        packet.origin_timestamp
    );
    debug!(
        "| Receive timestamp   (server):\t{:>16}",
        packet.recv_timestamp
    );
    debug!(
        "| Transmit timestamp  (server):\t{:>16}",
        packet.tx_timestamp
    );
    debug!("| Receive timestamp   (client):\t{:>16}", recv_timestamp);
    debug!(
        "| Reference timestamp (server):\t{:>16}",
        packet.ref_timestamp
    );
    debug!("{}", delimiter_gen());
}

fn get_ntp_timestamp<T: NtpTimestampGenerator>(timestamp_gen: &T) -> u64 {
    ((timestamp_gen.timestamp_sec()
        + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
        << 32)
        + u64::from(timestamp_gen.timestamp_subsec_micros())
            * u64::from(u32::MAX)
            / u64::from(USEC_IN_SEC)
}

/// Convert second fraction value to milliseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_milliseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(MSEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to microseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_microseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(USEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to nanoseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_nanoseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(NSEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to picoseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_picoseconds(sec_fraction: u32) -> u64 {
    (u128::from(sec_fraction) * u128::from(PSEC_IN_SEC) / u128::from(u32::MAX))
        as u64
}

#[cfg(test)]
mod sntpc_ntp_result_tests {
    use crate::types::Units;
    use crate::{
        fraction_to_microseconds, fraction_to_milliseconds,
        fraction_to_nanoseconds, fraction_to_picoseconds, offset_calculate,
        NtpResult,
    };

    struct Timestamps(u64, u64, u64, u64);
    struct OffsetCalcTestCase {
        timestamp: Timestamps,
        expected: i64,
    }

    impl OffsetCalcTestCase {
        fn new(t1: u64, t2: u64, t3: u64, t4: u64, expected: i64) -> Self {
            OffsetCalcTestCase {
                timestamp: Timestamps(t1, t2, t3, t4),
                expected,
            }
        }

        fn t1(&self) -> u64 {
            self.timestamp.0
        }

        fn t2(&self) -> u64 {
            self.timestamp.1
        }

        fn t3(&self) -> u64 {
            self.timestamp.2
        }

        fn t4(&self) -> u64 {
            self.timestamp.3
        }
    }

    #[test]
    fn test_ntp_result() {
        let result1 = NtpResult::new(0, 0, 0, 0, 1, -2);

        assert_eq!(0, result1.sec());
        assert_eq!(0, result1.sec_fraction());
        assert_eq!(0, result1.roundtrip());
        assert_eq!(0, result1.offset());
        assert_eq!(1, result1.stratum());
        assert_eq!(-2, result1.precision());

        let result2 = NtpResult::new(1, 2, 3, 4, 5, -23);

        assert_eq!(1, result2.sec());
        assert_eq!(2, result2.sec_fraction());
        assert_eq!(3, result2.roundtrip());
        assert_eq!(4, result2.offset());
        assert_eq!(5, result2.stratum());
        assert_eq!(-23, result2.precision());

        let result3 =
            NtpResult::new(u32::MAX - 1, u32::MAX, u64::MAX, i64::MAX, 1, -127);

        assert_eq!(u32::MAX, result3.sec());
        assert_eq!(0, result3.sec_fraction());
        assert_eq!(u64::MAX, result3.roundtrip());
        assert_eq!(i64::MAX, result3.offset());
        assert_eq!(-127, result3.precision());
    }

    #[test]
    fn test_ntp_fraction_overflow_result() {
        let result = NtpResult::new(0, u32::MAX, 0, 0, 1, -19);
        assert_eq!(1, result.sec());
        assert_eq!(0, result.sec_fraction());
        assert_eq!(0, result.roundtrip());
        assert_eq!(0, result.offset());

        let result = NtpResult::new(u32::MAX - 1, u32::MAX, 0, 0, 1, -17);
        assert_eq!(u32::MAX, result.sec());
        assert_eq!(0, result.sec_fraction());
        assert_eq!(0, result.roundtrip());
        assert_eq!(0, result.offset());
    }

    #[test]
    fn test_conversion_to_ms() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
        assert_eq!(999u32, milliseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let milliseconds = fraction_to_milliseconds(result.seconds_fraction);
        assert_eq!(0u32, milliseconds);
    }

    #[test]
    fn test_conversion_to_us() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let microseconds = fraction_to_microseconds(result.seconds_fraction);
        assert_eq!(999_999u32, microseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let microseconds = fraction_to_microseconds(result.seconds_fraction);
        assert_eq!(0u32, microseconds);
    }

    #[test]
    fn test_conversion_to_ns() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
        assert_eq!(999_999_999u32, nanoseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
        assert_eq!(0u32, nanoseconds);
    }

    #[test]
    fn test_conversion_to_ps() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(999_999_999_767u64, picoseconds);

        let result = NtpResult::new(0, 1, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(232u64, picoseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(0u64, picoseconds);
    }

    #[test]
    fn test_offset_calculate() {
        let tests = [
            OffsetCalcTestCase::new(
                16_893_142_954_672_769_962,
                16_893_142_959_053_084_959,
                16_893_142_959_053_112_968,
                16_893_142_954_793_063_406,
                1_005_870,
            ),
            OffsetCalcTestCase::new(
                16_893_362_966_131_575_843,
                16_893_362_966_715_800_791,
                16_893_362_966_715_869_584,
                16_893_362_967_084_349_913,
                25115,
            ),
            OffsetCalcTestCase::new(
                16_893_399_716_399_327_198,
                16_893_399_716_453_045_029,
                16_893_399_716_453_098_083,
                16_893_399_716_961_924_964,
                -52981,
            ),
            OffsetCalcTestCase::new(
                9_487_534_663_484_046_772u64,
                16_882_120_099_581_835_046u64,
                16_882_120_099_583_884_144u64,
                9_487_534_663_651_464_597u64,
                1_721_686_086_620_926,
            ),
        ];

        for t in tests {
            let offset = offset_calculate(
                t.t1(),
                t.t2(),
                t.t3(),
                t.t4(),
                Units::Microseconds,
            );
            let expected = t.expected;
            assert_eq!(offset, expected);
        }
    }

    #[test]
    fn test_units_str_representation() {
        assert_eq!(format!("{}", Units::Milliseconds), "ms");
        assert_eq!(format!("{}", Units::Microseconds), "us");
    }
}

#[cfg(all(test, feature = "std", feature = "std-socket", feature = "sync"))]
mod sntpc_sync_tests {
    use crate::sync::get_time;
    use crate::{Error, NtpContext, StdTimestampGen};
    use std::net::{ToSocketAddrs, UdpSocket};

    #[test]
    fn test_ntp_request_sntpv4_supported() {
        let context = NtpContext::new(StdTimestampGen::default());
        let pools = [
            "pool.ntp.org:123",
            "time.google.com:123",
            "time.apple.com:123",
            "time.cloudflare.com:123",
            "time.facebook.com:123",
        ];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");

            for address in pool.to_socket_addrs().unwrap() {
                let result = get_time(address, &socket, context);

                assert!(
                    result.is_ok(),
                    "{} is bad - {:?}",
                    pool,
                    result.unwrap_err()
                );
                assert_ne!(result.unwrap().seconds, 0);
            }
        }
    }

    #[test]
    fn test_ntp_request_sntpv3_not_supported() {
        let context = NtpContext::new(StdTimestampGen::default());

        let pools = ["time.nist.gov:123", "time.windows.com:123"];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");

            for address in pool.to_socket_addrs().unwrap() {
                let result = get_time(address, &socket, context);
                assert!(result.is_err(), "{pool} is ok");
                assert_eq!(
                    result.unwrap_err(),
                    Error::IncorrectResponseVersion
                );
            }
        }
    }

    #[test]
    fn test_invalid_addrs_ntp_request() {
        let pool = "asdf.asdf.asdf:123";
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .expect("Unable to set up socket timeout");

        let result = pool.to_socket_addrs();
        assert!(result.is_err(), "{pool} is ok");
    }
}

#[cfg(all(test, feature = "std", feature = "std-socket"))]
mod sntpc_async_tests {
    use crate::get_time;
    use crate::{Error, NtpContext, StdTimestampGen};
    use miniloop::executor::Executor;
    use std::net::{ToSocketAddrs, UdpSocket};

    #[test]
    fn test_ntp_async_request_sntpv4_supported() {
        let context = NtpContext::new(StdTimestampGen::default());
        let pools = [
            "pool.ntp.org:123",
            "time.google.com:123",
            "time.apple.com:123",
            "time.cloudflare.com:123",
            "time.facebook.com:123",
        ];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");

            for address in pool.to_socket_addrs().unwrap() {
                let result = Executor::new()
                    .block_on(get_time(address, &socket, context));

                assert!(
                    result.is_ok(),
                    "{} is bad - {:?}",
                    pool,
                    result.unwrap_err()
                );
                assert_ne!(result.unwrap().seconds, 0);
            }
        }
    }

    #[test]
    fn test_ntp_async_request_sntpv3_not_supported() {
        let context = NtpContext::new(StdTimestampGen::default());

        let pools = ["time.nist.gov:123", "time.windows.com:123"];

        for pool in &pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");

            for address in pool.to_socket_addrs().unwrap() {
                let result = Executor::new()
                    .block_on(get_time(address, &socket, context));
                assert!(result.is_err(), "{pool} is ok");
                assert_eq!(
                    result.unwrap_err(),
                    Error::IncorrectResponseVersion
                );
            }
        }
    }
}
