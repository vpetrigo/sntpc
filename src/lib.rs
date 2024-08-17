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
//! sntpc = "0.3.7"
//! ```
//!
//! ## Features
//!
//! Sntpc supports several features:
//! - `std`: includes functionality that depends on the standard library
//! - `utils`: includes functionality that mostly OS specific and allows system time sync
//! - `log`: enables library debug output during execution
//! - `async`: enables asynchronous feature support
//!
//! <div class="example-wrap" style="display:inline-block"><pre class="compile_fail" style="white-space:normal;font:inherit;">
//!
//! **Warning**: `utils` feature is not stable and may change in the future.
//! </pre></div>
//!
//! # Details
//!
//! There are multiple approaches how the library can be used:
//! - under environments where a networking stuff is hidden in system/RTOS kernel, [`get_time`] can
//! be used since it encapsulates network I/O
//! - under environments where TCP/IP stack requires to call some helper functions like `poll`,
//! `wait`, etc. and/or there are no options to perform I/O operations within a single call,
//! [`sntp_send_request`] and [`sntp_process_response`] can be used
//!
//! As `sntpc` supports `no_std` environment as well, it was
//! decided to provide a set of traits to implement for a network object (`UdpSocket`)
//! and timestamp generator:
//! - [`NtpUdpSocket`] trait should be implemented for `UdpSocket`-like objects for the
//! library to be able to send and receive data from NTP servers
//! - [`NtpTimestampGenerator`] trait should be implemented for timestamp generator objects to
//! provide the library with system related timestamps
//!
//! ## Logging support
//!
//! Library debug logs can be enabled in executables by enabling `log` feature. Server
//! addresses, response payload will be printed.
//!
//! # Example
//!
//! ```rust
//! # #[cfg(not(feature = "std"))]
//! # use no_std_net::{SocketAddr, ToSocketAddrs, IpAddr, Ipv4Addr};
//! # #[cfg(feature = "std")]
//! use std::net::UdpSocket;
//! use std::time::Duration;
//!
//! # #[cfg(not(feature = "std"))]
//! # #[derive(Debug)]
//! # struct UdpSocket;
//! # #[cfg(not(feature = "std"))]
//! # impl UdpSocket {
//! #     fn bind(addr: &str) -> sntpc::Result<Self> {
//! #         Ok(UdpSocket)
//! #     }
//! #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> sntpc::Result<usize> {
//! #         Ok(0usize)
//! #     }
//! #     fn recv_from(&self, buf: &mut [u8]) -> sntpc::Result<(usize, SocketAddr)> {
//! #         Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
//! #     }
//! #     fn set_read_timeout<T>(&self, _arg: T) -> sntpc::Result<()> {
//! #         Ok(())
//! #     }
//! # }
//!
//! fn main() {
//!     let socket =
//!         UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
//!     socket
//!        .set_read_timeout(Some(Duration::from_secs(2)))
//!        .expect("Unable to set UDP socket read timeout");
//!     # #[cfg(all(feature = "std"))]
//!     let result = sntpc::simple_get_time("time.google.com:123", socket);
//!     # #[cfg(all(feature = "std"))]
//!     match result {
//!        Ok(time) => {
//!            println!("Got time: {}.{}", time.sec(), sntpc::fraction_to_milliseconds(time.sec_fraction()));
//!        }
//!        Err(err) => println!("Err: {:?}", err),
//!     }
//!  }
//! ```
//!
//! For more complex example with custom timestamp generator and UDP socket implementation, see
//! `examples/smoltcp_request.rs`.
//!
//! For usage SNTP-client in an asynchronous environment, see `examples/tokio.rs`
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "utils")]
pub mod utils;

mod types;
pub use crate::types::*;

#[cfg(feature = "async")]
pub mod async_impl;

use core::fmt::Debug;
use core::iter::Iterator;
use core::marker::Copy;
use core::mem;

pub(crate) mod net {
    #[cfg(not(feature = "std"))]
    pub use no_std_net::{SocketAddr, ToSocketAddrs};

    #[cfg(feature = "std")]
    pub use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
}

#[cfg(feature = "log")]
use log::debug;

/// Send request to a NTP server with the given address and process the response in a single call
///
/// May be useful under an environment with `std` networking implementation, where all
/// network stuff is hidden within system's kernel. For environment with custom
/// Uses [`NtpUdpSocket`] and [`NtpTimestampGenerator`] trait bounds to allow generic specification
/// of objects that can be used with the library
/// **Args:**
/// - `pool_addrs` - Server's name or IP address with port specification as a string
/// - `socket` - UDP socket object that will be used during NTP request-response
/// communication
/// - `context` - SNTP client context to provide timestamp generation feature
///
/// # Example
///
/// ```rust
/// use sntpc::{self, NtpContext, NtpTimestampGenerator, Result};
/// use std::time::Duration;
/// # #[cfg(not(feature = "std"))]
/// # use no_std_net::{SocketAddr, ToSocketAddrs, IpAddr, Ipv4Addr};
/// # #[cfg(feature = "std")]
/// # use std::net::{SocketAddr, ToSocketAddrs, UdpSocket, IpAddr, Ipv4Addr};
/// # #[cfg(not(feature = "std"))]
/// # #[derive(Debug)]
/// # struct UdpSocket;
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket)
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(0usize)
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// // implement required trait on network objects
/// # #[derive(Debug)]
/// # struct UdpSocketWrapper(UdpSocket);
/// #
/// # impl sntpc::NtpUdpSocket for UdpSocketWrapper {
/// #     fn send_to<T: ToSocketAddrs>(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: T,
/// #     ) -> Result<usize> {
/// #         match self.0.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.0.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
/// // implement required trait on the timestamp generator object
/// #[derive(Copy, Clone, Default)]
/// struct StdTimestampGen {
///     duration: Duration,
/// }
///
/// impl NtpTimestampGenerator for StdTimestampGen {
///     fn init(&mut self) {
///         self.duration = std::time::SystemTime::now()
///             .duration_since(std::time::SystemTime::UNIX_EPOCH)
///             .unwrap();
///     }
///
///     fn timestamp_sec(&self) -> u64 {
///         self.duration.as_secs()
///     }
///
///     fn timestamp_subsec_micros(&self) -> u32 {
///         self.duration.subsec_micros()
///     }
/// }
///
/// let ntp_context = NtpContext::new(StdTimestampGen::default());
/// let socket = UdpSocketWrapper(UdpSocket::bind("0.0.0.0:0").expect("something"));
/// # #[cfg(feature = "std")]
/// let result = sntpc::get_time("time.google.com:123", socket, ntp_context);
/// // OR
/// // let result = sntpc::get_time("83.168.200.199:123", socket, context);
///
/// // .. process the result
/// ```
pub fn get_time<A, U, T>(
    pool_addrs: A,
    socket: U,
    context: NtpContext<T>,
) -> Result<NtpResult>
where
    A: net::ToSocketAddrs + Copy + Debug,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    let result = sntp_send_request(pool_addrs, &socket, context)?;

    sntp_process_response(pool_addrs, &socket, context, result)
}

#[cfg(feature = "std")]
/// Supplementary `get_time` alternative that wraps provided UDP socket into a wrapper type
/// that implements necessary traits for SNTP client proper operation
pub fn simple_get_time<A>(
    pool_addrs: A,
    socket: net::UdpSocket,
) -> Result<NtpResult>
where
    A: net::ToSocketAddrs + Copy + Debug,
{
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    get_time(pool_addrs, socket, ntp_context)
}

/// Send SNTP request to a server
///
/// That function along with the [`sntp_process_response`] is required under an environment(s)
/// where you need to call TCP/IP stack helpers (like `poll`, `wait`, etc.)
/// *Args*:
/// - `dest` - Initial NTP server's address to validate response against
/// - `socket` - Socket reference to use for receiving a NTP response
/// - `context` - SNTP client context
///
/// # Example
///
/// ```
/// use sntpc::{self, NtpContext, NtpTimestampGenerator, Result};
/// # use std::time::Duration;
/// # use std::str::FromStr;
/// # #[cfg(not(feature = "std"))]
/// # use no_std_net::{SocketAddr, ToSocketAddrs, IpAddr, Ipv4Addr};
/// # #[cfg(feature = "std")]
/// # use std::net::{SocketAddr, ToSocketAddrs, UdpSocket, IpAddr, Ipv4Addr};
/// # #[cfg(not(feature = "std"))]
/// # #[derive(Debug)]
/// # struct UdpSocket(u8);
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket(0))
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(0usize)
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// // implement required trait on network objects
/// # #[derive(Debug)]
/// # struct UdpSocketWrapper(UdpSocket);
///
/// # impl sntpc::NtpUdpSocket for UdpSocketWrapper {
/// #     fn send_to<T: ToSocketAddrs>(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: T,
/// #     ) -> Result<usize> {
/// #         match self.0.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.0.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
/// // implement required trait on the timestamp generator object
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
/// #
/// # let ntp_context = NtpContext::new(StdTimestampGen::default());
/// # let socket = UdpSocketWrapper(UdpSocket::bind("0.0.0.0:0").expect("something"));
/// // "time.google.com:123" string here used for the sake of simplicity. In the real app
/// // you would want to fix destination address, since string hostname may resolve to
/// // different IP addresses
/// # #[cfg(feature = "std")]
/// let result = sntpc::sntp_send_request("time.google.com:123", &socket, ntp_context);
/// ```
pub fn sntp_send_request<A, U, T>(
    dest: A,
    socket: &U,
    context: NtpContext<T>,
) -> Result<SendRequestResult>
where
    A: net::ToSocketAddrs + Debug,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    #[cfg(feature = "log")]
    debug!("Address: {:?}, Socket: {:?}", dest, *socket);
    let request = NtpPacket::new(context.timestamp_gen);

    send_request(dest, &request, socket)?;
    Ok(SendRequestResult::from(request))
}

/// Process SNTP response from a server
///
/// That function along with the [`sntp_send_request`] is required under an environment(s)
/// where you need to call TCP/IP stack helpers (like `poll`, `wait`, etc.)
/// *Args*:
/// - `dest` - NTP server's address to send request to
/// - `socket` - Socket reference to use for sending a NTP request
/// - `context` - SNTP client context
/// - `send_req_result` - send request result that obtained after [`sntp_send_request`] call
///
/// # Example
/// ```
/// use sntpc::{self, NtpContext, NtpTimestampGenerator, Result};
/// # use std::time::Duration;
/// # use std::str::FromStr;
/// # #[cfg(not(feature = "std"))]
/// # use no_std_net::{SocketAddr, ToSocketAddrs, IpAddr, Ipv4Addr};
/// # #[cfg(feature = "std")]
/// # use std::net::{SocketAddr, ToSocketAddrs, UdpSocket, IpAddr, Ipv4Addr};
/// # #[cfg(not(feature = "std"))]
/// # #[derive(Debug, Clone)]
/// # struct UdpSocket(u8);
/// # #[cfg(not(feature = "std"))]
/// # impl UdpSocket {
/// #     fn bind(addr: &str) -> Result<Self> {
/// #         Ok(UdpSocket(0))
/// #     }
/// #     fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], dest: T) -> Result<usize> {
/// #        Ok(0usize)
/// #     }
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #        Ok((0usize, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)))
/// #     }
/// # }
/// // implement required trait on network objects
/// # #[derive(Debug)]
/// # struct UdpSocketWrapper(UdpSocket);
/// #
/// # impl sntpc::NtpUdpSocket for UdpSocketWrapper {
/// #     fn send_to<T: ToSocketAddrs>(
/// #         &self,
/// #         buf: &[u8],
/// #         addr: T,
/// #     ) -> Result<usize> {
/// #         match self.0.send_to(buf, addr) {
/// #             Ok(usize) => Ok(usize),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// #
/// #     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
/// #         match self.0.recv_from(buf) {
/// #             Ok((size, addr)) => Ok((size, addr)),
/// #             Err(_) => Err(sntpc::Error::Network),
/// #         }
/// #     }
/// # }
/// // implement required trait on the timestamp generator object
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
/// #
/// # let ntp_context = NtpContext::new(StdTimestampGen::default());
/// # let socket = UdpSocketWrapper(UdpSocket::bind("0.0.0.0:0").expect("something"));
/// // "time.google.com:123" string here used for the sake of simplicity. In the real app
/// // you would want to fix destination address, since string hostname may resolve to
/// // different IP addresses
/// # #[cfg(feature = "std")]
/// if let Ok(result) = sntpc::sntp_send_request("time.google.com:123", &socket, ntp_context) {
///     let time = sntpc::sntp_process_response("time.google.com:123", &socket, ntp_context, result);
/// }
/// ```
pub fn sntp_process_response<A, U, T>(
    dest: A,
    socket: &U,
    mut context: NtpContext<T>,
    send_req_result: SendRequestResult,
) -> Result<NtpResult>
where
    A: net::ToSocketAddrs + Debug,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    let mut response_buf = RawNtpPacket::default();
    let (response, src) = socket.recv_from(response_buf.0.as_mut())?;
    context.timestamp_gen.init();
    let recv_timestamp = get_ntp_timestamp(context.timestamp_gen);
    #[cfg(feature = "log")]
    debug!("Response: {}", response);

    match dest.to_socket_addrs() {
        Err(_) => return Err(Error::AddressResolve),
        Ok(mut it) => {
            if !it.any(|addr| addr == src) {
                return Err(Error::ResponseAddressMismatch);
            }
        }
    }

    if response != mem::size_of::<NtpPacket>() {
        return Err(Error::IncorrectPayload);
    }

    let result =
        process_response(send_req_result, response_buf, recv_timestamp);

    if let Ok(_r) = &result {
        #[cfg(feature = "log")]
        debug!("{:?}", _r);
    }

    result
}

fn send_request<A: net::ToSocketAddrs, U: NtpUdpSocket>(
    dest: A,
    req: &NtpPacket,
    socket: &U,
) -> core::result::Result<(), Error> {
    let buf = RawNtpPacket::from(req);

    match socket.send_to(&buf.0, dest) {
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
    fn ntohl<T: NtpNum>(val: T) -> T::Type {
        val.ntohl()
    }

    packet.root_delay = ntohl(packet.root_delay);
    packet.root_dispersion = ntohl(packet.root_dispersion);
    packet.ref_id = ntohl(packet.ref_id);
    packet.ref_timestamp = ntohl(packet.ref_timestamp);
    packet.origin_timestamp = ntohl(packet.origin_timestamp);
    packet.recv_timestamp = ntohl(packet.recv_timestamp);
    packet.tx_timestamp = ntohl(packet.tx_timestamp);
}

fn convert_delays(sec: u64, fraction: u64, units: u64) -> u64 {
    sec * units + fraction * units / u32::MAX as u64
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
        Units::Milliseconds => {
            convert_delays(delta_sec, delta_sec_fraction, MSEC_IN_SEC as u64)
        }
        Units::Microseconds => {
            convert_delays(delta_sec, delta_sec_fraction, USEC_IN_SEC as u64)
        }
    }
}

fn offset_calculate(t1: u64, t2: u64, t3: u64, t4: u64, units: Units) -> i64 {
    let theta = ((t2.wrapping_sub(t1) / 2) as i64)
        .saturating_add((t3.wrapping_sub(t4) / 2) as i64);
    let theta_sec = (theta.unsigned_abs() & SECONDS_MASK) >> 32;
    let theta_sec_fraction = theta.unsigned_abs() & SECONDS_FRAC_MASK;

    match units {
        Units::Milliseconds => {
            convert_delays(theta_sec, theta_sec_fraction, MSEC_IN_SEC as u64)
                as i64
                * theta.signum()
        }
        Units::Microseconds => {
            convert_delays(theta_sec, theta_sec_fraction, USEC_IN_SEC as u64)
                as i64
                * theta.signum()
        }
    }
}

#[test]
fn test_offset_calculate() {
    let t1 = 9487534663484046772u64;
    let t2 = 16882120099581835046u64;
    let t3 = 16882120099583884144u64;
    let t4 = 9487534663651464597u64;

    assert_eq!(
        offset_calculate(t1, t2, t3, t4, Units::Microseconds),
        1721686086620926
    );
}

#[cfg(feature = "log")]
fn debug_ntp_packet(packet: &NtpPacket, _recv_timestamp: u64) {
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);

    use core::str;

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
    debug!("| Receive timestamp   (client):\t{:>16}", _recv_timestamp);
    debug!(
        "| Reference timestamp (server):\t{:>16}",
        packet.ref_timestamp
    );
    debug!("{}", delimiter_gen());
}

fn get_ntp_timestamp<T: NtpTimestampGenerator>(timestamp_gen: T) -> u64 {
    ((timestamp_gen.timestamp_sec()
        + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
        << 32)
        + timestamp_gen.timestamp_subsec_micros() as u64 * u32::MAX as u64
            / USEC_IN_SEC as u64
}

/// Convert second fraction value to milliseconds value
pub fn fraction_to_milliseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(MSEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to microseconds value
pub fn fraction_to_microseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(USEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to nanoseconds value
pub fn fraction_to_nanoseconds(sec_fraction: u32) -> u32 {
    (u64::from(sec_fraction) * u64::from(NSEC_IN_SEC) / u64::from(u32::MAX))
        as u32
}

/// Convert second fraction value to picoseconds value
pub fn fraction_to_picoseconds(sec_fraction: u32) -> u64 {
    (u128::from(sec_fraction) * u128::from(PSEC_IN_SEC) / u128::from(u32::MAX))
        as u64
}

#[cfg(test)]
mod sntpc_ntp_result_tests {
    use crate::{
        fraction_to_microseconds, fraction_to_milliseconds,
        fraction_to_nanoseconds, fraction_to_picoseconds, NtpResult,
    };

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
        assert_eq!(999999u32, microseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let microseconds = fraction_to_microseconds(result.seconds_fraction);
        assert_eq!(0u32, microseconds);
    }

    #[test]
    fn test_conversion_to_ns() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
        assert_eq!(999999999u32, nanoseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let nanoseconds = fraction_to_nanoseconds(result.seconds_fraction);
        assert_eq!(0u32, nanoseconds);
    }

    #[test]
    fn test_conversion_to_ps() {
        let result = NtpResult::new(0, u32::MAX - 1, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(999999999767u64, picoseconds);

        let result = NtpResult::new(0, 1, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(232u64, picoseconds);

        let result = NtpResult::new(0, 0, 0, 0, 1, 0);
        let picoseconds = fraction_to_picoseconds(result.seconds_fraction);
        assert_eq!(0u64, picoseconds);
    }
}

#[cfg(all(test, feature = "std"))]
mod sntpc_tests {
    use crate::{get_time, Error, NtpContext, StdTimestampGen, Units};
    use std::net::UdpSocket;

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

            let result = get_time(pool, socket, context);

            assert!(
                result.is_ok(),
                "{} is bad - {:?}",
                pool,
                result.unwrap_err()
            );
            assert_ne!(result.unwrap().seconds, 0);
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
            let result = get_time(pool, socket, context);
            assert!(result.is_err(), "{} is ok", pool);
            assert_eq!(result.unwrap_err(), Error::IncorrectResponseVersion);
        }
    }

    #[test]
    fn test_invalid_addrs_ntp_request() {
        let context = NtpContext::new(StdTimestampGen::default());
        let pool = "asdf.asdf.asdf:123";
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .expect("Unable to set up socket timeout");

        let result = get_time(pool, socket, context);
        assert!(result.is_err(), "{} is ok", pool);
        assert_eq!(result.unwrap_err(), Error::Network);
    }

    #[test]
    fn test_units_str_representation() {
        assert_eq!(format!("{}", Units::Milliseconds), "ms");
        assert_eq!(format!("{}", Units::Microseconds), "us");
    }
}
