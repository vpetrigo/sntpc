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
//! sntpc = "0.3"
//! ```
//!
//! ## Features
//!
//! Sntpc supports several features:
//! - `std`: includes functionality that depends on the standard library
//! - `utils`: includes functionality that mostly OS specific and allows system time sync
//! - `log`: enables library debug output during execution
//! </div><div class="example-wrap" style="display:inline-block"><pre class="compile_fail" style="white-space:normal;font:inherit;">
//!
//! **Warning**: `utils` feature is not stable and may change in the future.
//!
//! </pre></div>
//!
//! # Details
//!
//! Currently there is a single method to issue SNTP requests to a server of interest
//! [`request_with_addrs`]. As `sntpc` supports `no_std` environment as well, it was
//! decided to provide a set of traits to implement for a network object (`UdpSocket`)
//! and timestamp generator:
//! - [`NtpUdpSocket`] trait should be implemented for `UdpSocket`-like objects for the
//! library to be able to send and receive data from NTP servers
//! - [`NtpTimestamp`] trait should be implemented for timestamp generator objects to
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
//! use sntpc::{Error, NtpContext, NtpTimestamp, NtpUdpSocket};
//! use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
//! use std::time::Duration;
//!
//! #[derive(Copy, Clone, Default)]
//! struct StdTimestampGen {
//!     duration: Duration,
//! }
//!
//! impl NtpTimestamp for StdTimestampGen {
//!     fn init(&mut self) {
//!         self.duration = std::time::SystemTime::now()
//!             .duration_since(std::time::SystemTime::UNIX_EPOCH)
//!             .unwrap();
//!     }
//!
//!     fn timestamp_sec(&self) -> u64 {
//!         self.duration.as_secs()
//!     }
//!
//!     fn timestamp_subsec_micros(&self) -> u32 {
//!         self.duration.subsec_micros()
//!     }
//! }
//!
//! #[derive(Debug)]
//! struct UdpSocketWrapper(UdpSocket);
//!
//! impl NtpUdpSocket for UdpSocketWrapper {
//!     fn send_to<T: ToSocketAddrs>(
//!         &self,
//!         buf: &[u8],
//!         addr: T,
//!     ) -> Result<usize, Error> {
//!         match self.0.send_to(buf, addr) {
//!             Ok(usize) => Ok(usize),
//!             Err(_) => Err(Error::Network),
//!         }
//!     }
//!
//!     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
//!         match self.0.recv_from(buf) {
//!             Ok((size, addr)) => Ok((size, addr)),
//!             Err(_) => Err(Error::Network),
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let socket =
//!         UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
//!     socket
//!        .set_read_timeout(Some(Duration::from_secs(2)))
//!        .expect("Unable to set UDP socket read timeout");
//!     let sock_wrapper = UdpSocketWrapper(socket);
//!     let ntp_context = NtpContext::new(StdTimestampGen::default());
//!     let result =
//!         sntpc::request_with_addrs("time.google.com:123", sock_wrapper, ntp_context);
//!
//!     match result {
//!        Ok(time) => {
//!            println!("Got time: {}.{}", time.sec(), time.nsec());
//!        }
//!        Err(err) => println!("Err: {:?}", err),
//!     }
//!  }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "utils")]
pub mod utils;

use core::fmt::Debug;
use core::iter::Iterator;
use core::marker::Copy;
use core::mem;

mod net {
    #[cfg(not(feature = "std"))]
    pub use no_std_net::{SocketAddr, ToSocketAddrs};

    #[cfg(feature = "std")]
    pub use std::net::{SocketAddr, ToSocketAddrs};
}

#[cfg(feature = "log")]
use log::debug;

/// SNTP mode value bit mask
const MODE_MASK: u8 = 0b0000_0111;
/// SNTP mode bit mask shift value
const MODE_SHIFT: u8 = 0;
/// SNTP version value bit mask
const VERSION_MASK: u8 = 0b0011_1000;
/// SNTP mode bit mask shift value
const VERSION_SHIFT: u8 = 3;
/// SNTP LI (leap indicator) bit mask value
const LI_MASK: u8 = 0b1100_0000;
/// SNTP LI bit mask shift value
const LI_SHIFT: u8 = 6;
/// SNTP nanoseconds in second constant
const NSEC_IN_SEC: u32 = 1_000_000_000;

#[derive(Debug)]
struct NtpPacket {
    li_vn_mode: u8,
    stratum: u8,
    poll: i8,
    precision: i8,
    root_delay: u32,
    root_dispersion: u32,
    ref_id: u32,
    ref_timestamp: u64,
    origin_timestamp: u64,
    recv_timestamp: u64,
    tx_timestamp: u64,
}

/// The error type for SNTP client
/// Errors originate on network layer or during processing response from a NTP server
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Origin timestamp value in a NTP response differs from the value
    /// that has been sent in the NTP request
    IncorrectOriginTimestamp,
    /// Incorrect mode value in a NTP response
    IncorrectMode,
    /// Incorrect Leap Indicator (LI) value in a NTP response
    IncorrectLeapIndicator,
    /// Incorrect version in a NTP response. Currently SNTPv4 is supported
    IncorrectResponseVersion,
    /// Incorrect stratum headers in a NTP response
    IncorrectStratumHeaders,
    /// Payload size of a NTP response does not meet SNTPv4 specification
    IncorrectPayload,
    /// Network error occurred.
    Network,
    /// A NTP server address can not be resolved
    AddressResolve,
    /// A NTP server address response has been received from does not match
    /// to the address the request was sent to
    ResponseAddressMismatch,
}

/// SNTP request result representation
#[derive(Debug)]
pub struct NtpResult {
    /// NTP server seconds value
    pub sec: u32,
    /// NTP server nanoseconds value
    pub nsec: u32,
    /// Request roundtrip time
    pub roundtrip: u64,
    /// Offset of the current system time with one received from a NTP server
    pub offset: i64,
}

impl NtpResult {
    /// Create new NTP result
    /// Args:
    /// * `sec` - number of seconds
    /// * `nsec` - number of nanoseconds
    /// * `roundtrip` - calculated roundtrip in microseconds
    /// * `offset` - calculated system clock offset in microseconds
    pub fn new(sec: u32, nsec: u32, roundtrip: u64, offset: i64) -> Self {
        let residue = nsec / NSEC_IN_SEC;
        let nsec = nsec % NSEC_IN_SEC;
        let sec = sec + residue;

        NtpResult {
            sec,
            nsec,
            roundtrip,
            offset,
        }
    }
    /// Returns number of seconds reported by an NTP server
    pub fn sec(&self) -> u32 {
        self.sec
    }

    /// Returns number of nanoseconds reported by an NTP server
    pub fn nsec(&self) -> u32 {
        self.nsec
    }

    /// Returns request's roundtrip time (client -> server -> client) in microseconds
    pub fn roundtrip(&self) -> u64 {
        self.roundtrip
    }

    /// Returns system clock offset value in microseconds
    pub fn offset(&self) -> i64 {
        self.offset
    }
}

impl NtpPacket {
    const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;

    pub fn new<T: NtpTimestamp>(mut timestamp_gen: T) -> NtpPacket {
        timestamp_gen.init();
        let tx_timestamp = get_ntp_timestamp(timestamp_gen);

        #[cfg(feature = "log")]
        debug!(target: "NtpPacket::new", "{}", tx_timestamp);

        NtpPacket {
            li_vn_mode: NtpPacket::SNTP_CLIENT_MODE | NtpPacket::SNTP_VERSION,
            stratum: 0,
            poll: 0,
            precision: 0,
            root_delay: 0,
            root_dispersion: 0,
            ref_id: 0,
            ref_timestamp: 0,
            origin_timestamp: 0,
            recv_timestamp: 0,
            tx_timestamp,
        }
    }
}

/// A trait encapsulating timestamp generator's operations
///
/// Since under `no_std` environment `time::now()` implementations may be not available,
/// you can implement that trait on an object you want and provide proper system
/// timestamps for the SNTP client. According to specs, all timestamps calculated from
/// UNIX EPOCH "_1970-01-01 00:00:00 UTC_"
pub trait NtpTimestamp {
    /// Initialize timestamp generator state with `now` system time since UNIX EPOCH.
    /// Expected to be called every time before `timestamp_sec` and
    /// `timestamp_subsec_micros` usage. Basic flow would be the following:
    ///
    /// ```ignore
    /// # Timestamp A required
    /// init()
    /// timestamp_sec()
    /// timestamp_subsec_micros()
    /// // ...
    /// # Timestamp B required
    /// init()
    /// timestamp_sec()
    /// timestamp_subsec_micros()
    /// // ... so on
    /// ```
    fn init(&mut self);

    /// Returns timestamp in seconds since UNIX EPOCH for the initialized generator
    fn timestamp_sec(&self) -> u64;

    /// Returns the fractional part of the timestamp in whole micro seconds.
    /// That method **should not** return microseconds since UNIX EPOCH
    fn timestamp_subsec_micros(&self) -> u32;
}

/// A trait encapsulating UDP socket interface required for SNTP client operations
pub trait NtpUdpSocket {
    /// Send the given buffer to an address provided. On success, returns the number
    /// of bytes written.
    ///
    /// Since multiple SocketAddr objects can hide behind the type (domain name can be
    /// resolved to multiple addresses), the method should send data to a single address
    /// available in `addr`
    fn send_to<T: net::ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> core::result::Result<usize, Error>;

    /// Receives a single datagram message on the socket. On success, returns the number
    /// of bytes read and the origin.
    ///
    /// The function will be called with valid byte array `buf` of sufficient size to
    /// hold the message bytes
    fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> core::result::Result<(usize, net::SocketAddr), Error>;
}

/// SNTP client context that contains of objects that may be required for client's
/// operation
#[derive(Copy, Clone)]
pub struct NtpContext<T: NtpTimestamp> {
    pub timestamp_gen: T,
}

impl<T: NtpTimestamp + Copy> NtpContext<T> {
    pub fn new(timestamp_gen: T) -> Self {
        NtpContext { timestamp_gen }
    }
}

trait NtpNum {
    type Type;

    fn ntohl(&self) -> Self::Type;
}

impl NtpNum for u32 {
    type Type = u32;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}

impl NtpNum for u64 {
    type Type = u64;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}

struct RawNtpPacket([u8; mem::size_of::<NtpPacket>()]);

impl Default for RawNtpPacket {
    fn default() -> Self {
        RawNtpPacket([0u8; mem::size_of::<NtpPacket>()])
    }
}

impl From<RawNtpPacket> for NtpPacket {
    fn from(val: RawNtpPacket) -> Self {
        // left it here for a while, maybe in future Rust releases there
        // will be a way to use such a generic function with compile-time
        // size determination
        // const fn to_array<T: Sized>(x: &[u8]) -> [u8; mem::size_of::<T>()] {
        //     let mut temp_buf = [0u8; mem::size_of::<T>()];
        //
        //     temp_buf.copy_from_slice(x);
        //     temp_buf
        // }
        let to_array_u32 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u32>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };
        let to_array_u64 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u64>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };

        NtpPacket {
            li_vn_mode: val.0[0],
            stratum: val.0[1],
            poll: val.0[2] as i8,
            precision: val.0[3] as i8,
            root_delay: u32::from_le_bytes(to_array_u32(&val.0[4..8])),
            root_dispersion: u32::from_le_bytes(to_array_u32(&val.0[8..12])),
            ref_id: u32::from_le_bytes(to_array_u32(&val.0[12..16])),
            ref_timestamp: u64::from_le_bytes(to_array_u64(&val.0[16..24])),
            origin_timestamp: u64::from_le_bytes(to_array_u64(&val.0[24..32])),
            recv_timestamp: u64::from_le_bytes(to_array_u64(&val.0[32..40])),
            tx_timestamp: u64::from_le_bytes(to_array_u64(&val.0[40..48])),
        }
    }
}

impl From<&NtpPacket> for RawNtpPacket {
    fn from(val: &NtpPacket) -> Self {
        let mut tmp_buf = [0u8; mem::size_of::<NtpPacket>()];

        tmp_buf[0] = val.li_vn_mode;
        tmp_buf[1] = val.stratum;
        tmp_buf[2] = val.poll as u8;
        tmp_buf[3] = val.precision as u8;
        tmp_buf[4..8].copy_from_slice(&val.root_delay.to_be_bytes());
        tmp_buf[8..12].copy_from_slice(&val.root_dispersion.to_be_bytes());
        tmp_buf[12..16].copy_from_slice(&val.ref_id.to_be_bytes());
        tmp_buf[16..24].copy_from_slice(&val.ref_timestamp.to_be_bytes());
        tmp_buf[24..32].copy_from_slice(&val.origin_timestamp.to_be_bytes());
        tmp_buf[32..40].copy_from_slice(&val.recv_timestamp.to_be_bytes());
        tmp_buf[40..48].copy_from_slice(&val.tx_timestamp.to_be_bytes());

        RawNtpPacket(tmp_buf)
    }
}

/// Send request to a NTP server with the given address and process the response
///
/// Uses [`NtpUdpSocket`] and [`NtpTimestamp`] trait bounds to allow generic specification
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
/// use sntpc;
/// use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};
/// use std::time::Duration;
/// use sntpc::NtpContext;
/// // implement required trait on network objects
/// #[derive(Debug)]
/// struct UdpSocketWrapper(UdpSocket);
///
/// impl sntpc::NtpUdpSocket for UdpSocketWrapper {
///     fn send_to<T: ToSocketAddrs>(
///         &self,
///         buf: &[u8],
///         addr: T,
///     ) -> Result<usize, sntpc::Error> {
///         match self.0.send_to(buf, addr) {
///             Ok(usize) => Ok(usize),
///             Err(_) => Err(sntpc::Error::Network),
///         }
///     }
///
///     fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), sntpc::Error> {
///         match self.0.recv_from(buf) {
///             Ok((size, addr)) => Ok((size, addr)),
///             Err(_) => Err(sntpc::Error::Network),
///         }
///     }
/// }
/// // implement required trait on the timestamp generator object
/// #[derive(Copy, Clone, Default)]
/// struct StdTimestampGen {
///     duration: Duration,
/// }
///
/// impl sntpc::NtpTimestamp for StdTimestampGen {
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
/// let result = sntpc::request_with_addrs("time.google.com:123", socket, context);
/// // OR
/// // let result = sntpc::request_with_addrs("83.168.200.199:123", socket, context);
///
/// // .. process the result
/// ```
pub fn request_with_addrs<A, U, T>(
    pool_addrs: A,
    socket: U,
    mut context: NtpContext<T>,
) -> core::result::Result<NtpResult, Error>
where
    A: net::ToSocketAddrs + Copy + Debug,
    U: NtpUdpSocket + Debug,
    T: NtpTimestamp + Copy,
{
    #[cfg(feature = "log")]
    debug!("Address: {:?}, Socket: {:?}", pool_addrs, socket);

    let request = NtpPacket::new(context.timestamp_gen);
    let dest = pool_addrs;

    if let Err(err) = send_request(dest, &request, &socket) {
        return Err(err);
    }

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

    let result = process_response(&request, response_buf, recv_timestamp);

    return match result {
        Ok(result) => {
            #[cfg(feature = "log")]
            debug!("{:?}", result);
            Ok(result)
        }
        Err(err) => Err(err),
    };
}

fn send_request<A: net::ToSocketAddrs, U: NtpUdpSocket>(
    dest: A,
    req: &NtpPacket,
    socket: &U,
) -> core::result::Result<(), Error> {
    let buf = RawNtpPacket::from(req);

    return match socket.send_to(&buf.0, dest) {
        Ok(size) => {
            if size == buf.0.len() {
                Ok(())
            } else {
                Err(Error::Network)
            }
        }
        Err(_) => Err(Error::Network),
    };
}

fn process_response(
    req: &NtpPacket,
    resp: RawNtpPacket,
    recv_timestamp: u64,
) -> Result<NtpResult, Error> {
    const SNTP_UNICAST: u8 = 4;
    const SNTP_BROADCAST: u8 = 5;
    const LI_MAX_VALUE: u8 = 3;
    const MSEC_MASK: u64 = 0x0000_0000_ffff_ffff;
    let shifter = |val, mask, shift| (val & mask) >> shift;
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);
    #[cfg(debug_assertions)]
    debug_ntp_packet(&packet);

    if req.tx_timestamp != packet.origin_timestamp {
        return Err(Error::IncorrectOriginTimestamp);
    }
    // Shift is 0
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let resp_version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let req_version = shifter(req.li_vn_mode, VERSION_MASK, VERSION_SHIFT);

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
    let delta = (recv_timestamp - packet.origin_timestamp) as i64
        - (packet.tx_timestamp - packet.recv_timestamp) as i64;
    let theta = ((packet.recv_timestamp as i64
        - packet.origin_timestamp as i64)
        + (packet.tx_timestamp as i64 - recv_timestamp as i64))
        / 2;

    #[cfg(feature = "log")]
    debug!("Roundtrip delay: {} us. Offset: {} us", delta.abs(), theta);

    let seconds = (packet.tx_timestamp >> 32) as u32;
    let nsec = (packet.tx_timestamp & MSEC_MASK) as u32;
    let tx_tm = seconds - NtpPacket::NTP_TIMESTAMP_DELTA;

    Ok(NtpResult::new(tx_tm, nsec, delta.abs() as u64, theta))
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

#[cfg(debug_assertions)]
fn debug_ntp_packet(packet: &NtpPacket) {
    let shifter = |val, mask, shift| (val & mask) >> shift;
    #[allow(unused)]
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    #[allow(unused)]
    let version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    #[allow(unused)]
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);

    #[cfg(feature = "log")]
    {
        use core::str;

        let delimiter_gen = || {
            unsafe {
                str::from_utf8_unchecked(&[b'='; 52])
            }
        };

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
        debug!("| Reference timestamp:\t{:>16}", packet.ref_timestamp);
        debug!("| Origin timestamp:\t\t{:>16}", packet.origin_timestamp);
        debug!("| Receive timestamp:\t\t{:>16}", packet.recv_timestamp);
        debug!("| Transmit timestamp:\t\t{:>16}", packet.tx_timestamp);
        debug!("{}", delimiter_gen());
    }
}

fn get_ntp_timestamp<T: NtpTimestamp>(timestamp_gen: T) -> u64 {
    let timestamp = ((timestamp_gen.timestamp_sec()
        + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
        << 32)
        + u64::from(timestamp_gen.timestamp_subsec_micros());

    timestamp
}

#[cfg(test)]
mod sntpc_ntp_result_tests {
    use crate::{NtpResult, NSEC_IN_SEC};

    #[test]
    fn test_ntp_result() {
        let result1 = NtpResult::new(0, 0, 0, 0);

        assert_eq!(0, result1.sec());
        assert_eq!(0, result1.nsec());
        assert_eq!(0, result1.roundtrip());
        assert_eq!(0, result1.offset());

        let result2 = NtpResult::new(1, 2, 3, 4);

        assert_eq!(1, result2.sec());
        assert_eq!(2, result2.nsec());
        assert_eq!(3, result2.roundtrip());
        assert_eq!(4, result2.offset());

        let residue3 = u32::MAX / NSEC_IN_SEC;
        let result3 =
            NtpResult::new(u32::MAX - residue3, u32::MAX, u64::MAX, i64::MAX);

        assert_eq!(u32::MAX, result3.sec());
        assert_eq!(u32::MAX % NSEC_IN_SEC, result3.nsec());
        assert_eq!(u64::MAX, result3.roundtrip());
        assert_eq!(i64::MAX, result3.offset());
    }

    #[test]
    fn test_ntp_nsec_overflow_result() {
        let result = NtpResult::new(0, u32::MAX, 0, 0);
        let max_value_sec = u32::MAX / NSEC_IN_SEC;
        let max_value_nsec = u32::MAX % NSEC_IN_SEC;

        assert_eq!(max_value_sec, result.sec());
        assert_eq!(max_value_nsec, result.nsec());
        assert_eq!(0, result.roundtrip());
        assert_eq!(0, result.offset());
    }
}

#[cfg(all(test, feature = "std"))]
mod sntpc_tests {
    use crate::net::{SocketAddr, ToSocketAddrs};
    use crate::{
        request_with_addrs, Error, NtpContext, NtpTimestamp, NtpUdpSocket,
    };
    use std::net::UdpSocket;

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

        fn recv_from(
            &self,
            buf: &mut [u8],
        ) -> Result<(usize, SocketAddr), Error> {
            match self.recv_from(buf) {
                Ok((size, addr)) => Ok((size, addr)),
                Err(_) => Err(Error::Network),
            }
        }
    }

    #[derive(Copy, Clone, Default)]
    struct StdTimestampGen(std::time::Duration);

    impl NtpTimestamp for StdTimestampGen {
        fn init(&mut self) {
            self.0 = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap();
        }

        fn timestamp_sec(&self) -> u64 {
            self.0.as_secs()
        }

        fn timestamp_subsec_micros(&self) -> u32 {
            self.0.subsec_micros()
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
            "stratum1.net:123",
        ];

        for pool in pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");

            let result = request_with_addrs(pool, socket, context);

            assert!(
                result.is_ok(),
                "{} is bad - {:?}",
                pool,
                result.unwrap_err()
            );
            assert_ne!(result.unwrap().sec, 0);
        }
    }

    #[test]
    fn test_ntp_request_sntpv3_not_supported() {
        let context = NtpContext::new(StdTimestampGen::default());

        let pools = ["time.nist.gov:123", "time.windows.com:123"];

        for pool in pools {
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("Unable to set up socket timeout");
            let result = request_with_addrs(pool, socket, context);
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

        let result = request_with_addrs(pool, socket, context);
        assert!(result.is_err(), "{} is ok", pool);
        assert_eq!(result.unwrap_err(), Error::Network);
    }
}
