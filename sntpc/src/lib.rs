//! Rust SNTP client implementation
//!
//! # Overview
//!
//! This crate provides an async-first SNTP client for sending requests to NTP servers
//! and processing responses to extract accurate timestamps.
//!
//! Supported protocol version: [SNTPv4 (RFC 4330)](https://datatracker.ietf.org/doc/html/rfc4330)
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! sntpc = "0.8"
//! ```
//!
//! For common usage patterns, choose a network adapter:
//! - `sntpc-net-std` - Standard library UDP sockets
//! - `sntpc-net-tokio` - Tokio async runtime
//! - `sntpc-net-embassy` - Embassy embedded runtime
//!
//! ## Features
//!
//! - `std` - Standard library support (includes [`StdTimestampGen`])
//! - `sync` - Synchronous API in [`sync`] module (default is async)
//! - `utils` - OS-specific utilities for system time sync ⚠️ **Unstable API**
//! - `log` - Debug logging via `log` crate
//! - `defmt` - Debug logging via `defmt` (mutually exclusive with `log`)
//!
//! <div class="warning">
//!
//! **Warning**: `log` and `defmt` are mutually exclusive features. If both are enabled,
//! `defmt` takes priority.
//! </div>
//!
//! ## Architecture
//!
//! The library is designed to work in both `std` and `no_std` environments through two key traits:
//! - [`NtpUdpSocket`] - Implement for your UDP socket type
//! - [`NtpTimestampGenerator`] - Implement for your timestamp source
//!
//! For `std` environments, [`StdTimestampGen`] is provided.
//!
//! ### API Approaches
//!
//! - [`get_time`] - Complete request/response in a single call (suitable for most cases)
//! - [`sntp_send_request`] and [`sntp_process_response`] - Split send/receive workflow
//!   (useful when the TCP/IP stack requires polling or has custom timing requirements)
//!
//! ## Examples
//!
//! See the [examples directory](https://github.com/vpetrigo/sntpc/tree/master/examples) for complete examples:
//! - `simple-request` - Basic synchronous usage
//! - `tokio` - Async with tokio runtime
//! - `embassy-net` - Embedded async with embassy
//! - `smoltcp-request` - Custom `no_std` networking
//! - And more...
//!
//! Refer to individual function documentation for minimal code examples
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "utils")]
pub mod utils;

mod log;
mod types;

pub use crate::types::*;

#[cfg(any(feature = "log", feature = "defmt"))]
use crate::log::debug;

use cfg_if::cfg_if;

use core::net;

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
///   sending/receiving UDP packets.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator that implements
///   the [`NtpTimestampGenerator`] trait. This ensures precise timestamp creation for request and response processing.
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
/// Basic usage with standard library components:
///
/// ```no_run
/// use sntpc::{get_time, NtpContext, StdTimestampGen};
/// use std::net::SocketAddr;
///
/// # #[cfg(feature = "std")]
/// # async fn example() -> sntpc::Result<()> {
/// use sntpc_net_std::UdpSocketWrapper;
/// use std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to bind socket");
/// let socket = UdpSocketWrapper::new(socket);
/// let context = NtpContext::new(StdTimestampGen::default());
/// let addr: SocketAddr = "time.google.com:123".parse().unwrap();
///
/// let result = get_time(addr, &socket, context).await?;
/// println!("Time: {}.{}", result.sec(), result.sec_fraction());
/// # Ok(())
/// # }
/// ```
///
/// For custom implementations of [`NtpUdpSocket`] and [`NtpTimestampGenerator`],
/// see the examples in the repository, particularly `examples/smoltcp-request`
///
/// # Errors
///
/// This function returns an `Err` in any of the following cases:
/// * The SNTP packet could not be sent to the server.
/// * The response payload is invalid or indicates an error.
/// * Mismatch between the expected and actual server addresses.
pub async fn get_time<U, T>(addr: net::SocketAddr, socket: &U, context: NtpContext<T>) -> Result<NtpResult>
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
///   that is used to send/receive UDP packets.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator
///   that implements the [`NtpTimestampGenerator`] trait to provide a custom mechanism for generating timestamps.
///
/// # Returns
///
/// Returns a `Result<SendRequestResult>`:
/// * `Ok(SendRequestResult)` - If the packet was successfully sent, includes details
///   about the request, such as the originate timestamp.
/// * `Err(Error)` - If there was an error in sending the request, such as a network failure.
///
/// # Examples
///
/// For most use cases, prefer [`get_time`] which handles both sending and receiving.
/// Use this function directly only when you need split send/receive workflow:
///
/// ```no_run
/// use sntpc::{sntp_send_request, sntp_process_response, NtpContext, StdTimestampGen};
/// use std::net::SocketAddr;
///
/// # #[cfg(feature = "std")]
/// # async fn example() -> sntpc::Result<()> {
/// use sntpc_net_std::UdpSocketWrapper;
/// use std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to bind socket");
/// let socket = UdpSocketWrapper::new(socket);
/// let context = NtpContext::new(StdTimestampGen::default());
/// let addr: SocketAddr = "time.google.com:123".parse().unwrap();
///
/// let request_result = sntp_send_request(addr, &socket, context).await?;
/// // ... custom processing or polling here ...
/// let response = sntp_process_response(addr, &socket, context, request_result).await?;
/// # Ok(())
/// # }
/// ```
///
/// For custom implementations, see `examples/smoltcp-request`
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
    #[cfg(any(feature = "log", feature = "defmt"))]
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
///   used for receiving the response.
/// * `context` - An SNTP context (`NtpContext<T>`) containing a timestamp generator
///   that manages internal time calculations.
/// * `send_req_result` - The result of the previously sent request, containing the originate timestamp
///   of the SNTP request.
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
/// This function is typically used in conjunction with [`sntp_send_request`]:
///
/// ```no_run
/// use sntpc::{sntp_send_request, sntp_process_response, NtpContext, StdTimestampGen};
/// use std::net::SocketAddr;
///
/// # #[cfg(feature = "std")]
/// # async fn example() -> sntpc::Result<()> {
/// use sntpc_net_std::UdpSocketWrapper;
/// use std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to bind socket");
/// let socket = UdpSocketWrapper::new(socket);
/// let context = NtpContext::new(StdTimestampGen::default());
/// let addr: SocketAddr = "time.google.com:123".parse().unwrap();
///
/// let request_result = sntp_send_request(addr, &socket, context).await?;
/// let response = sntp_process_response(addr, &socket, context, request_result).await?;
///
/// println!("Offset: {} µs, Roundtrip: {} µs", response.offset(), response.roundtrip());
/// # Ok(())
/// # }
/// ```
///
/// For complete examples, see [`get_time`] or `examples/smoltcp-request`
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
    #[cfg(any(feature = "log", feature = "defmt"))]
    debug!("Response: {}", response);

    if dest != src {
        return Err(Error::ResponseAddressMismatch);
    }

    if response != size_of::<NtpPacket>() {
        return Err(Error::IncorrectPayload);
    }

    let result = process_response(send_req_result, response_buf, recv_timestamp);

    #[cfg(any(feature = "log", feature = "defmt"))]
    if let Ok(r) = &result {
        debug!("{:?}", r);
    }

    result
}

async fn send_request<U>(dest: net::SocketAddr, req: &NtpPacket, socket: &U) -> Result<()>
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
    #[cfg(any(feature = "log", feature = "defmt"))]
    use crate::log::debug;
    use crate::net;
    use crate::types::{NtpContext, NtpResult, NtpTimestampGenerator, NtpUdpSocket, Result, SendRequestResult};
    pub(crate) const SYNC_EXECUTOR_NUMBER_OF_TASKS: usize = 1;

    use miniloop::executor::Executor;
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
    pub fn get_time<U, T>(addr: net::SocketAddr, socket: &U, context: NtpContext<T>) -> Result<NtpResult>
    where
        U: NtpUdpSocket,
        T: NtpTimestampGenerator + Copy,
    {
        let result = sntp_send_request(addr, socket, context)?;
        #[cfg(any(feature = "log", feature = "defmt"))]
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
    ///   assists in generating timestamps for the request.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if the underlying async SNTP request fails for any reason,
    /// such as network failure, invalid server response, or timeout.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sntpc::sync::sntp_send_request;
    /// use sntpc::{NtpContext, StdTimestampGen};
    /// use std::net::SocketAddr;
    ///
    /// # #[cfg(feature = "std")]
    /// # fn example() -> sntpc::Result<()> {
    /// use sntpc_net_std::UdpSocketWrapper;
    /// use std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to bind socket");
    /// let socket = UdpSocketWrapper::new(socket);
    /// let context = NtpContext::new(StdTimestampGen::default());
    /// let addr: SocketAddr = "time.google.com:123".parse().unwrap();
    ///
    /// let request_result = sntp_send_request(addr, &socket, context)?;
    /// println!("Request sent with timestamp: {:?}", request_result);
    /// # Ok(())
    /// # }
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
        Executor::<1>::new().block_on(crate::sntp_send_request(dest, socket, context))
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
    /// ```no_run
    /// use sntpc::sync::{sntp_send_request, sntp_process_response};
    /// use sntpc::{NtpContext, StdTimestampGen};
    /// use std::net::SocketAddr;
    ///
    /// # #[cfg(feature = "std")]
    /// # fn example() -> sntpc::Result<()> {
    /// use sntpc_net_std::UdpSocketWrapper;
    /// use std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to bind socket");
    /// let socket = UdpSocketWrapper::new(socket);
    /// let context = NtpContext::new(StdTimestampGen::default());
    /// let addr: SocketAddr = "time.google.com:123".parse().unwrap();
    ///
    /// let request_result = sntp_send_request(addr, &socket, context)?;
    /// let ntp_result = sntp_process_response(addr, &socket, context, request_result)?;
    ///
    /// println!("NTP Result: {:?}", ntp_result);
    /// # Ok(())
    /// # }
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
        Executor::<SYNC_EXECUTOR_NUMBER_OF_TASKS>::new().block_on(crate::sntp_process_response(
            dest,
            socket,
            context,
            send_req_result,
        ))
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn process_response(send_req_result: SendRequestResult, resp: RawNtpPacket, recv_timestamp: u64) -> Result<NtpResult> {
    const SNTP_UNICAST: u8 = 4;
    const SNTP_BROADCAST: u8 = 5;
    const LI_MAX_VALUE: u8 = 3;
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);

    cfg_if!(
        if #[cfg(any(feature = "log", feature = "defmt"))] {
            let debug_packet = DebugNtpPacket::new(&packet, recv_timestamp);
            debug!("{:#?}", debug_packet);
        }
    );

    if send_req_result.originate_timestamp != packet.origin_timestamp {
        return Err(Error::IncorrectOriginTimestamp);
    }
    // Shift is 0
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let resp_version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let req_version = shifter(send_req_result.version, VERSION_MASK, VERSION_SHIFT);

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

    #[cfg(any(feature = "log", feature = "defmt"))]
    debug!("Roundtrip delay: {} {}. Offset: {} {}", roundtrip, units, offset, units);

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

fn roundtrip_calculate(t1: u64, t2: u64, t3: u64, t4: u64, units: Units) -> u64 {
    let delta = t4.wrapping_sub(t1).saturating_sub(t3.wrapping_sub(t2));
    let delta_sec = (delta & SECONDS_MASK) >> 32;
    let delta_sec_fraction = delta & SECONDS_FRAC_MASK;

    match units {
        Units::Milliseconds => convert_delays(delta_sec, delta_sec_fraction, u64::from(MSEC_IN_SEC)),
        Units::Microseconds => convert_delays(delta_sec, delta_sec_fraction, u64::from(USEC_IN_SEC)),
    }
}

#[allow(clippy::cast_possible_wrap)]
fn offset_calculate(t1: u64, t2: u64, t3: u64, t4: u64, units: Units) -> i64 {
    let theta = (t2.wrapping_sub(t1) as i64 / 2).saturating_add(t3.wrapping_sub(t4) as i64 / 2);
    let theta_sec = (theta.unsigned_abs() & SECONDS_MASK) >> 32;
    let theta_sec_fraction = theta.unsigned_abs() & SECONDS_FRAC_MASK;

    match units {
        Units::Milliseconds => {
            convert_delays(theta_sec, theta_sec_fraction, u64::from(MSEC_IN_SEC)) as i64 * theta.signum()
        }
        Units::Microseconds => {
            convert_delays(theta_sec, theta_sec_fraction, u64::from(USEC_IN_SEC)) as i64 * theta.signum()
        }
    }
}

fn get_ntp_timestamp<T: NtpTimestampGenerator>(timestamp_gen: &T) -> u64 {
    ((timestamp_gen.timestamp_sec() + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA))) << 32)
        + u64::from(timestamp_gen.timestamp_subsec_micros()) * u64::from(u32::MAX) / u64::from(USEC_IN_SEC)
}

/// Convert second fraction value to milliseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_milliseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(MSEC_IN_SEC) / u64::from(u32::MAX)) as u32
}

/// Convert second fraction value to microseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_microseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(USEC_IN_SEC) / u64::from(u32::MAX)) as u32
}

/// Convert second fraction value to nanoseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_nanoseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(NSEC_IN_SEC) / u64::from(u32::MAX)) as u32
}

/// Convert second fraction value to picoseconds value
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fraction_to_picoseconds(sec_fraction: u32) -> u64 {
    (u128::from(sec_fraction) * u128::from(PSEC_IN_SEC) / u128::from(u32::MAX)) as u64
}

#[cfg(test)]
mod sntpc_ntp_result_tests {
    use crate::types::Units;
    use crate::{
        NtpResult, fraction_to_microseconds, fraction_to_milliseconds, fraction_to_nanoseconds,
        fraction_to_picoseconds, offset_calculate,
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

        let result3 = NtpResult::new(u32::MAX - 1, u32::MAX, u64::MAX, i64::MAX, 1, -127);

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
            let offset = offset_calculate(t.t1(), t.t2(), t.t3(), t.t4(), Units::Microseconds);
            let expected = t.expected;
            assert_eq!(offset, expected);
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod sntpc_std_tests {
    use crate::types::Units;

    #[test]
    fn test_units_str_representation() {
        assert_eq!(format!("{}", Units::Milliseconds), "ms");
        assert_eq!(format!("{}", Units::Microseconds), "us");
    }
}

#[cfg(all(test, feature = "std", feature = "sync"))]
mod sntpc_sync_tests {
    use crate::sync::get_time;
    use crate::{Error, NtpContext, NtpUdpSocket, StdTimestampGen};
    use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

    struct UdpSocketWrapper {
        socket: UdpSocket,
    }

    impl UdpSocketWrapper {
        #[must_use]
        fn new(socket: UdpSocket) -> Self {
            Self { socket }
        }
    }

    impl From<UdpSocket> for UdpSocketWrapper {
        fn from(socket: UdpSocket) -> Self {
            UdpSocketWrapper::new(socket)
        }
    }

    impl NtpUdpSocket for UdpSocketWrapper {
        async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> crate::Result<usize> {
            match self.socket.send_to(buf, addr) {
                Ok(usize) => Ok(usize),
                Err(_) => Err(Error::Network),
            }
        }

        async fn recv_from(&self, buf: &mut [u8]) -> crate::Result<(usize, SocketAddr)> {
            match self.socket.recv_from(buf) {
                Ok((size, addr)) => Ok((size, addr)),
                Err(_) => Err(Error::Network),
            }
        }
    }

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
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result = get_time(address, &socket, context);

                assert!(result.is_ok(), "{} is bad - {:?}", pool, result.unwrap_err());
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
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("Unable to set up socket timeout");
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result = get_time(address, &socket, context);
                assert!(result.is_err(), "{pool} is ok");
                assert_eq!(result.unwrap_err(), Error::IncorrectResponseVersion);
            }
        }
    }

    #[test]
    fn test_invalid_addrs_ntp_request() {
        let pool = "asdf.asdf.asdf:123";
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("Unable to set up socket timeout");

        let result = pool.to_socket_addrs();
        assert!(result.is_err(), "{pool} is ok");
    }
}

#[cfg(all(test, feature = "std", feature = "sync"))]
mod sntpc_async_tests {
    use crate::sync::SYNC_EXECUTOR_NUMBER_OF_TASKS;
    use crate::{Error, NtpContext, StdTimestampGen};
    use crate::{NtpUdpSocket, get_time};
    use miniloop::executor::Executor;
    use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

    struct UdpSocketWrapper {
        socket: UdpSocket,
    }

    impl UdpSocketWrapper {
        #[must_use]
        fn new(socket: UdpSocket) -> Self {
            Self { socket }
        }
    }

    impl From<UdpSocket> for UdpSocketWrapper {
        fn from(socket: UdpSocket) -> Self {
            UdpSocketWrapper::new(socket)
        }
    }

    impl NtpUdpSocket for UdpSocketWrapper {
        async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> crate::Result<usize> {
            match self.socket.send_to(buf, addr) {
                Ok(usize) => Ok(usize),
                Err(_) => Err(Error::Network),
            }
        }

        async fn recv_from(&self, buf: &mut [u8]) -> crate::Result<(usize, SocketAddr)> {
            match self.socket.recv_from(buf) {
                Ok((size, addr)) => Ok((size, addr)),
                Err(_) => Err(Error::Network),
            }
        }
    }

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
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("Unable to set up socket timeout");
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result =
                    Executor::<SYNC_EXECUTOR_NUMBER_OF_TASKS>::new().block_on(get_time(address, &socket, context));

                assert!(result.is_ok(), "{pool} is bad - {:?}", result.unwrap_err());
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
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("Unable to set up socket timeout");
            let socket = UdpSocketWrapper::from(socket);

            for address in pool.to_socket_addrs().unwrap().filter(SocketAddr::is_ipv4) {
                let result =
                    Executor::<SYNC_EXECUTOR_NUMBER_OF_TASKS>::new().block_on(get_time(address, &socket, context));
                assert!(result.is_err(), "{pool} is ok");
                assert_eq!(result.unwrap_err(), Error::IncorrectResponseVersion);
            }
        }
    }
}
