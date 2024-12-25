use core::fmt::Formatter;
use core::fmt::{Debug, Display};
use core::mem;

use core::future::Future;
#[cfg(feature = "log")]
use log::debug;

use crate::get_ntp_timestamp;
use crate::net::SocketAddr;

/// SNTP mode value bit mask
pub(crate) const MODE_MASK: u8 = 0b0000_0111;
/// SNTP mode bit mask shift value
pub(crate) const MODE_SHIFT: u8 = 0;
/// SNTP version value bit mask
pub(crate) const VERSION_MASK: u8 = 0b0011_1000;
/// SNTP mode bit mask shift value
pub(crate) const VERSION_SHIFT: u8 = 3;
/// SNTP LI (leap indicator) bit mask value
pub(crate) const LI_MASK: u8 = 0b1100_0000;
/// SNTP LI bit mask shift value
pub(crate) const LI_SHIFT: u8 = 6;
/// SNTP picoseconds in second constant
pub(crate) const PSEC_IN_SEC: u64 = 1_000_000_000_000;
/// SNTP nanoseconds in second constant
pub(crate) const NSEC_IN_SEC: u32 = 1_000_000_000;
/// SNTP microseconds in second constant
pub(crate) const USEC_IN_SEC: u32 = 1_000_000;
/// SNTP milliseconds in second constant
pub(crate) const MSEC_IN_SEC: u32 = 1_000;
/// SNTP seconds mask
pub(crate) const SECONDS_MASK: u64 = 0xffff_ffff_0000_0000;
/// SNTP seconds fraction mask
pub(crate) const SECONDS_FRAC_MASK: u64 = 0xffff_ffff;

/// SNTP library result type
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct NtpPacket {
    pub(crate) li_vn_mode: u8,
    pub(crate) stratum: u8,
    pub(crate) poll: i8,
    pub(crate) precision: i8,
    pub(crate) root_delay: u32,
    pub(crate) root_dispersion: u32,
    pub(crate) ref_id: u32,
    pub(crate) ref_timestamp: u64,
    pub(crate) origin_timestamp: u64,
    pub(crate) recv_timestamp: u64,
    pub(crate) tx_timestamp: u64,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct NtpTimestamp {
    pub(crate) seconds: i64,
    pub(crate) seconds_fraction: i64,
}

impl From<u64> for NtpTimestamp {
    #[allow(clippy::cast_possible_wrap)]
    fn from(v: u64) -> Self {
        let seconds = (((v & SECONDS_MASK) >> 32)
            - u64::from(NtpPacket::NTP_TIMESTAMP_DELTA))
            as i64;
        let microseconds = (v & SECONDS_FRAC_MASK) as i64;

        NtpTimestamp {
            seconds,
            seconds_fraction: microseconds,
        }
    }
}

/// Helper enum for specification delay units
#[derive(Copy, Clone, Debug)]
pub(crate) enum Units {
    #[allow(dead_code)]
    Milliseconds,
    Microseconds,
}

impl Display for Units {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let unit = match self {
            Units::Microseconds => "us",
            Units::Milliseconds => "ms",
        };

        write!(f, "{unit}")
    }
}

/// The error type for SNTP client
/// Errors originate on network layer or during processing response from a NTP server
#[derive(Debug, PartialEq, Copy, Clone)]
#[non_exhaustive]
pub enum Error {
    /// Origin timestamp value in a NTP response differs from the value
    /// that has been sent in the NTP request
    IncorrectOriginTimestamp,
    /// Incorrect mode value in a NTP response
    IncorrectMode,
    /// Incorrect Leap Indicator (LI) value in a NTP response
    IncorrectLeapIndicator,
    /// Incorrect version in a NTP response. Currently, `SNTPv4` is supported
    IncorrectResponseVersion,
    /// Incorrect stratum headers in a NTP response
    IncorrectStratumHeaders,
    /// Payload size of a NTP response does not meet `SNTPv4` specification
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
#[derive(Debug, Copy, Clone)]
pub struct NtpResult {
    /// NTP server seconds value
    pub seconds: u32,
    /// NTP server seconds fraction value
    pub seconds_fraction: u32,
    /// Request roundtrip time in microseconds
    pub roundtrip: u64,
    /// Estimated difference between the NTP reference and the system time in microseconds
    pub offset: i64,
    /// Clock stratum of NTP server
    pub stratum: u8,
    /// Precision of NTP server as log2(seconds) - this should usually be negative
    pub precision: i8,
}

impl NtpResult {
    /// Create new NTP result
    /// Args:
    /// * `seconds` - number of seconds
    /// * `seconds_fraction` - number of seconds fraction
    /// * `roundtrip` - calculated roundtrip in microseconds
    /// * `offset` - calculated system clock offset in microseconds
    /// * `stratum` - integer indicating the stratum (level of server's hierarchy to stratum 0 - "reference clock")
    /// * `precision` - an exponent of two, where the resulting value is the precision of the system clock in seconds
    #[must_use]
    pub fn new(
        seconds: u32,
        seconds_fraction: u32,
        roundtrip: u64,
        offset: i64,
        stratum: u8,
        precision: i8,
    ) -> Self {
        let seconds = seconds + seconds_fraction / u32::MAX;
        let seconds_fraction = seconds_fraction % u32::MAX;

        NtpResult {
            seconds,
            seconds_fraction,
            roundtrip,
            offset,
            stratum,
            precision,
        }
    }
    /// Returns number of seconds reported by an NTP server
    #[must_use]
    pub fn sec(&self) -> u32 {
        self.seconds
    }

    /// Returns number of seconds fraction reported by an NTP server
    #[must_use]
    pub fn sec_fraction(&self) -> u32 {
        self.seconds_fraction
    }

    /// Returns request's roundtrip time (client -> server -> client) in microseconds
    #[must_use]
    pub fn roundtrip(&self) -> u64 {
        self.roundtrip
    }

    /// Returns system clock offset value in microseconds
    #[must_use]
    pub fn offset(&self) -> i64 {
        self.offset
    }

    /// Returns reported stratum value (level of server's hierarchy to stratum 0 - "reference clock")
    #[must_use]
    pub fn stratum(&self) -> u8 {
        self.stratum
    }

    /// Returns reported precision value (an exponent of two, which results in the precision of server's system clock in seconds)
    #[must_use]
    pub fn precision(&self) -> i8 {
        self.precision
    }
}

impl NtpPacket {
    // First day UNIX era offset https://www.rfc-editor.org/rfc/rfc5905
    pub(crate) const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;

    pub fn new<T: NtpTimestampGenerator>(mut timestamp_gen: T) -> NtpPacket {
        timestamp_gen.init();
        let tx_timestamp = get_ntp_timestamp(&timestamp_gen);

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
pub trait NtpTimestampGenerator {
    /// Initialize timestamp generator state with `now` system time since UNIX EPOCH.
    /// Expected to be called every time before `timestamp_sec` and
    /// `timestamp_subsec_micros` usage. Basic flow would be the following:
    ///
    /// ```text
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

#[cfg(feature = "std")]
/// Supplementary module to implement some `sntpc` boilerplate that environments with
/// `std` enable have to re-implement.
mod sup {
    use std::time::{Duration, SystemTime};

    use crate::NtpTimestampGenerator;

    /// Standard library timestamp generator wrapper type
    /// that relies on `std::time` to provide timestamps during SNTP client operations
    #[derive(Copy, Clone, Default)]
    pub struct StdTimestampGen {
        duration: Duration,
    }

    impl NtpTimestampGenerator for StdTimestampGen {
        fn init(&mut self) {
            self.duration = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap();
        }

        fn timestamp_sec(&self) -> u64 {
            self.duration.as_secs()
        }

        fn timestamp_subsec_micros(&self) -> u32 {
            self.duration.subsec_micros()
        }
    }
}

#[cfg(feature = "std")]
pub use sup::*;

/// A trait encapsulating UDP socket interface required for SNTP client operations
pub trait NtpUdpSocket {
    /// Send the given buffer to an address provided. On success, returns the number
    /// of bytes written.
    ///
    /// Since multiple `SocketAddr` objects can hide behind the type (domain name can be
    /// resolved to multiple addresses), the method should send data to a single address
    /// available in `addr`
    /// # Errors
    ///
    /// Will return `Err` if an underlying UDP send fails
    fn send_to(
        &self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> impl Future<Output = Result<usize>>;

    /// Receives a single datagram message on the socket. On success, returns the number
    /// of bytes read and the origin.
    ///
    /// The function will be called with valid byte array `buf` of sufficient size to
    /// hold the message bytes
    /// # Errors
    ///
    /// Will return `Err` if an underlying UDP receive fails
    fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>>;
}
// TODO: Clean up this
#[cfg(feature = "std")]
use std::net::UdpSocket;

#[cfg(feature = "std")]
impl NtpUdpSocket for UdpSocket {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        match self.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}

/// SNTP client context that contains of objects that may be required for client's
/// operation
#[derive(Copy, Clone)]
pub struct NtpContext<T: NtpTimestampGenerator> {
    pub timestamp_gen: T,
}

impl<T: NtpTimestampGenerator + Copy> NtpContext<T> {
    /// Create SNTP client context with the given timestamp generator
    pub fn new(timestamp_gen: T) -> Self {
        NtpContext { timestamp_gen }
    }
}

/// Preserve SNTP request sending operation result required during receiving and processing
/// state
#[derive(Copy, Clone, Debug)]
pub struct SendRequestResult {
    pub(crate) originate_timestamp: u64,
    pub(crate) version: u8,
}

impl From<NtpPacket> for SendRequestResult {
    fn from(ntp_packet: NtpPacket) -> Self {
        SendRequestResult {
            originate_timestamp: ntp_packet.tx_timestamp,
            version: ntp_packet.li_vn_mode,
        }
    }
}

pub(crate) trait NtpNum {
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

#[derive(Copy, Clone)]
pub(crate) struct RawNtpPacket(pub(crate) [u8; size_of::<NtpPacket>()]);

impl Default for RawNtpPacket {
    fn default() -> Self {
        RawNtpPacket([0u8; size_of::<NtpPacket>()])
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
            #[allow(clippy::cast_possible_wrap)]
            poll: val.0[2] as i8,
            #[allow(clippy::cast_possible_wrap)]
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
    #[allow(clippy::cast_sign_loss)]
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
