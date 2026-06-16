use crate::get_ntp_timestamp;
#[cfg(any(feature = "log", feature = "defmt"))]
use crate::log::debug;
use crate::net::SocketAddr;

use cfg_if::cfg_if;

use core::fmt::Formatter;
use core::fmt::{Debug, Display};
use core::future::Future;
use core::mem::size_of;

/// SNTP unicast mode constant
pub(crate) const SNTP_UNICAST: u8 = 4;
/// Maximum allowed root distance (16 seconds in NTP short format: 16 << 16)
pub(crate) const MAXDISP: u32 = 0x0010_0000;
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
/// SNTP LI unsynchronized value
pub(crate) const LI_UNSYNCHRONIZED: u8 = 3;
/// SNTP picoseconds in second constant
pub(crate) const PSEC_IN_SEC: u64 = 1_000_000_000_000;
/// RFC 5905 NTP header length in bytes.
pub(crate) const NTP_PACKET_LEN: usize = 48;
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

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

cfg_if! {
    if #[cfg(any(feature = "log", feature = "defmt"))] {
        use crate::shifter;

        use core::str;

        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub(crate) struct DebugNtpPacket<'a> {
            packet: &'a NtpPacket,
            client_recv_timestamp: u64,
        }

        impl<'a> DebugNtpPacket<'a> {
            pub(crate) fn new(
                packet: &'a NtpPacket,
                client_recv_timestamp: u64,
            ) -> Self {
                Self {
                    packet,
                    client_recv_timestamp,
                }
            }
        }

        impl Debug for DebugNtpPacket<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                let mode = shifter(self.packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
                let version =
                    shifter(self.packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
                let li = shifter(self.packet.li_vn_mode, LI_MASK, LI_SHIFT);
                let id_slice = &self.packet.ref_id.to_be_bytes();
                let reference_id = str::from_utf8(id_slice).unwrap_or("");

                f.debug_struct("NtpPacket")
                    .field("mode", &mode)
                    .field("version", &version)
                    .field("leap", &li)
                    .field("stratum", &self.packet.stratum)
                    .field("poll", &self.packet.poll)
                    .field("precision", &self.packet.precision)
                    .field("root delay", &self.packet.root_delay)
                    .field("root dispersion", &self.packet.root_dispersion)
                    .field("reference ID", &reference_id)
                    .field(
                        "origin timestamp (client)",
                        &self.packet.origin_timestamp,
                    )
                    .field(
                        "receive timestamp (server)",
                        &self.packet.recv_timestamp,
                    )
                    .field(
                        "transmit timestamp (server)",
                        &self.packet.tx_timestamp,
                    )
                    .field("receive timestamp (client)", &self.client_recv_timestamp)
                    .field("reference timestamp (server)", &self.packet.ref_timestamp)
                    .finish()
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct NtpTimestamp {
    pub(crate) seconds: i64,
    pub(crate) seconds_fraction: i64,
}

impl From<u64> for NtpTimestamp {
    #[allow(clippy::cast_possible_wrap)]
    fn from(v: u64) -> Self {
        let seconds = (((v & SECONDS_MASK) >> 32) - u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)) as i64;
        let microseconds = (v & SECONDS_FRAC_MASK) as i64;

        NtpTimestamp {
            seconds,
            seconds_fraction: microseconds,
        }
    }
}

/// Helper enum for specification delay units
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

/// Kiss-o'-Death code received from an NTP server (RFC 5905 §7.4).
///
/// When a server sends a packet with stratum 0, the Reference Identifier field
/// carries a 4-character ASCII kiss code. Different codes require different
/// client actions:
///
/// - `Deny` and `Rstr`: callers MUST stop using that server.
/// - `Rate`: callers SHOULD back off / reduce their polling interval.
/// - `Experimental`: codes beginning with 'X' are for unregistered
///   experimentation and should be ignored if unrecognized by the caller.
/// - `KoD` packets carry server-supplied timestamps, but callers MUST NOT rely
///   on them as normal time results.
/// - All other codes: No protocol significance; discard after inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum KissOfDeathCode {
    /// ACST: The association belongs to a unicast server
    Acst,
    /// AUTH: Server authentication failed
    Auth,
    /// AUTO: Autokey sequence failed
    Auto,
    /// BCST: The association belongs to a broadcast server
    Bcst,
    /// CRYP: Cryptographic authentication or identification failed
    Cryp,
    /// DENY: Access denied by remote server
    Deny,
    /// DROP: Lost peer in symmetric mode
    Drop,
    /// RSTR: Access denied due to local policy
    Rstr,
    /// INIT: The association has not yet synchronized for the first time
    Init,
    /// MCST: The association belongs to a dynamically discovered server
    Mcst,
    /// NKEY: No key found
    Nkey,
    /// RATE: Rate exceeded — client MUST reduce a polling interval
    Rate,
    /// RMOT: Alteration of association from a remote host
    Rmot,
    /// STEP: A step change in system time has occurred
    Step,
    /// Experimental/unregistered code (the first byte is 'X' per RFC 5905 §7.4)
    Experimental([u8; 4]),
}

impl KissOfDeathCode {
    /// Creates a new instance from a 4-byte code.
    ///
    /// If the bytes match a known RFC 5905 §7.4 kiss code, the corresponding
    /// variant is returned. Otherwise, the code is wrapped in `Experimental`.
    pub(crate) fn from_bytes(code: [u8; 4]) -> Self {
        match &code {
            b"ACST" => Self::Acst,
            b"AUTH" => Self::Auth,
            b"AUTO" => Self::Auto,
            b"BCST" => Self::Bcst,
            b"CRYP" => Self::Cryp,
            b"DENY" => Self::Deny,
            b"DROP" => Self::Drop,
            b"RSTR" => Self::Rstr,
            b"INIT" => Self::Init,
            b"MCST" => Self::Mcst,
            b"NKEY" => Self::Nkey,
            b"RATE" => Self::Rate,
            b"RMOT" => Self::Rmot,
            b"STEP" => Self::Step,
            _ => Self::Experimental(code),
        }
    }

    /// Returns the 4-character ASCII string representation of the kiss code.
    ///
    /// For known variants, the standard code string (e.g., `"DENY"`, `"RATE"`) is returned.
    /// For `Experimental` codes, the raw bytes are interpreted as UTF-8; if not valid UTF-8,
    /// an empty string (`""`) is returned.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Acst => "ACST",
            Self::Auth => "AUTH",
            Self::Auto => "AUTO",
            Self::Bcst => "BCST",
            Self::Cryp => "CRYP",
            Self::Deny => "DENY",
            Self::Drop => "DROP",
            Self::Rstr => "RSTR",
            Self::Init => "INIT",
            Self::Mcst => "MCST",
            Self::Nkey => "NKEY",
            Self::Rate => "RATE",
            Self::Rmot => "RMOT",
            Self::Step => "STEP",
            Self::Experimental(code) => str::from_utf8(code).unwrap_or(""),
        }
    }
}

/// The error type for SNTP client
/// Errors originate on network layer or during processing response from a NTP server
#[derive(Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Origin timestamp value in a NTP response differs from the value
    /// that has been sent in the NTP request
    IncorrectOriginTimestamp,
    /// Incorrect mode value in a NTP response
    IncorrectMode,
    /// Incorrect Leap Indicator (LI) value in a NTP response (LI=3 means clock unsynchronized)
    UnsynchronizedClock,
    /// NTP response contains an invalid timestamp (e.g., zero transmit timestamp)
    InvalidTimestamp,
    /// Root distance (`root_delay/2 + root_dispersion`) exceeds maximum allowed value (`MAXDISP`)
    ExcessiveRootDistance,
    /// Reference timestamp is newer than transmit timestamp, indicating invalid server data
    BackwardReferenceTimestamp,
    /// Incorrect version in a NTP response. Currently, `SNTPv4` is supported
    IncorrectResponseVersion,
    /// Incorrect stratum headers in a NTP response
    IncorrectStratumHeaders,
    /// Payload size of a NTP response does not meet `SNTPv4` specification
    IncorrectPayload,
    /// Network error occurred.
    Network,
    /// A NTP server address could not be resolved.
    ///
    /// This variant is provided for use by adapter crates that perform DNS resolution.
    /// The core `sntpc` library does not construct this error.
    AddressResolve,
    /// A NTP server address response has been received from does not match
    /// to the address the request was sent to
    ResponseAddressMismatch,
    /// Kiss-o'-Death packet received from an NTP server (RFC 5905 §7.4).
    ///
    /// The kiss code indicates the reason for the server's response. Callers MUST
    /// inspect the code and take appropriate action:
    ///
    /// - `KissOfDeathCode::Deny` and `KissOfDeathCode::Rstr`: stop using this server.
    /// - `KissOfDeathCode::Rate`: reduce/back off the polling interval.
    /// - `KissOfDeathCode::Experimental(_)`: unknown `X*` codes may be ignored if
    ///   unrecognized by the caller.
    /// - `KoD` packets must not be treated as normal time results; do not rely on
    ///   their timestamps for synchronization.
    KissOfDeath(KissOfDeathCode),
}

/// SNTP request result representation
///
/// # NTP Short Format
///
/// The `root_delay` and `root_dispersion` fields use NTP short format (16.16 fixed-point),
/// where the upper 16 bits represent seconds and the lower 16 bits represent a fraction
/// of a second (1/65536). To convert to seconds:
/// - `seconds = value >> 16`
/// - `fraction = value & 0xFFFF` (where fraction represents 1/65536 of a second)
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    /// Leap Indicator (LI) value from the server response
    /// - 0: no warning
    /// - 1: last minute has 61 seconds
    /// - 2: last minute has 59 seconds
    /// - 3: clock unsynchronized (alarm condition)
    pub leap_indicator: u8,
    /// Root delay in NTP short format (16-bit seconds + 16-bit fraction).
    /// To convert to seconds: `root_delay >> 16` gives the integer seconds,
    /// and `root_delay & 0xFFFF` gives the fractional part (1/65536 second units).
    pub root_delay: u32,
    /// Root dispersion in NTP short format (16-bit seconds + 16-bit fraction).
    /// To convert to seconds: `root_dispersion >> 16` gives the integer seconds,
    /// and `root_dispersion & 0xFFFF` gives the fractional part (1/65536 second units).
    pub root_dispersion: u32,
    /// Reference identifier in network byte order. Interpretation depends on stratum:
    /// - Stratum 0: Kiss-o'-Death ASCII code (4 characters)
    /// - Stratum 1: 4-character ASCII reference clock identifier (e.g., "GPS", "PPS")
    /// - Stratum 2+: IPv4 address (or first 4 bytes of MD5 hash of IPv6 address)
    pub reference_id: [u8; 4],
    /// Reference timestamp: time when the server's clock was last set or corrected,
    /// in NTP timestamp format (seconds since 1900-01-01 with 32-bit fraction)
    pub reference_timestamp: u64,
    /// Poll interval: maximum interval between successive messages, in log2 seconds
    pub poll: i8,
    /// Dispersion: estimated total dispersion in microseconds.
    /// When the `dispersion` feature is enabled, this is computed per RFC 5905 §9.2.
    /// When disabled, this is always 0.
    pub dispersion: u64,
}

impl NtpResult {
    /// Create new NTP result
    ///
    /// # Note
    ///
    /// The `seconds_fraction` value is preserved as provided.
    ///
    /// Args:
    /// * `seconds` - number of seconds
    /// * `seconds_fraction` - number of seconds fraction
    /// * `roundtrip` - calculated roundtrip in microseconds
    /// * `offset` - calculated system clock offset in microseconds
    /// * `stratum` - integer indicating the stratum (level of server's hierarchy to stratum 0 - "reference clock")
    /// * `precision` - an exponent of two, where the resulting value is the precision of the system clock in seconds
    /// * `leap_indicator` - leap indicator value from the server response (0-3)
    /// * `root_delay` - total round-trip delay to the reference clock in NTP short format (16.16 fixed-point)
    /// * `root_dispersion` - total dispersion to the reference clock in NTP short format (16.16 fixed-point)
    /// * `reference_id` - reference identifier as 4-byte array in network byte order
    /// * `reference_timestamp` - reference timestamp in NTP timestamp format
    /// * `poll` - poll interval in log2 seconds
    /// * `dispersion` - estimated total dispersion in microseconds
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seconds: u32,
        seconds_fraction: u32,
        roundtrip: u64,
        offset: i64,
        stratum: u8,
        precision: i8,
        leap_indicator: u8,
        root_delay: u32,
        root_dispersion: u32,
        reference_id: [u8; 4],
        reference_timestamp: u64,
        poll: i8,
        dispersion: u64,
    ) -> Self {
        NtpResult {
            seconds,
            seconds_fraction,
            roundtrip,
            offset,
            stratum,
            precision,
            leap_indicator,
            root_delay,
            root_dispersion,
            reference_id,
            reference_timestamp,
            poll,
            dispersion,
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

    /// Returns the leap indicator (LI) from the server response
    ///
    /// - 0: no warning
    /// - 1: last minute has 61 seconds
    /// - 2: last minute has 59 seconds
    /// - 3: clock unsynchronized (alarm condition)
    #[must_use]
    pub fn leap_indicator(&self) -> u8 {
        self.leap_indicator
    }

    /// Returns the root delay in NTP short format (16.16 fixed-point)
    #[must_use]
    pub fn root_delay(&self) -> u32 {
        self.root_delay
    }

    /// Returns the root dispersion in NTP short format (16.16 fixed-point)
    #[must_use]
    pub fn root_dispersion(&self) -> u32 {
        self.root_dispersion
    }

    /// Returns the reference identifier as a 4-byte array in network byte order.
    ///
    /// Interpretation depends on stratum:
    /// - Stratum 0: Kiss-o'-Death ASCII code
    /// - Stratum 1: 4-character ASCII reference clock identifier (e.g., "GPS", "PPS")
    /// - Stratum 2+: IPv4 address (or first 4 bytes of MD5 hash of IPv6 address)
    #[must_use]
    pub fn reference_id(&self) -> [u8; 4] {
        self.reference_id
    }

    /// Returns the reference timestamp in NTP timestamp format
    /// (seconds since 1900-01-01 with 32-bit fraction)
    #[must_use]
    pub fn reference_timestamp(&self) -> u64 {
        self.reference_timestamp
    }

    /// Returns the poll interval as log2 seconds
    #[must_use]
    pub fn poll(&self) -> i8 {
        self.poll
    }

    /// Returns the estimated total dispersion in microseconds
    #[must_use]
    pub fn dispersion(&self) -> u64 {
        self.dispersion
    }
}

impl NtpPacket {
    // First day UNIX era offset https://www.rfc-editor.org/rfc/rfc5905
    pub(crate) const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;

    pub fn new<T: NtpTimestampGenerator>(mut timestamp_gen: T) -> Self {
        timestamp_gen.init();
        let tx_timestamp = get_ntp_timestamp(&timestamp_gen);

        #[cfg(any(feature = "log", feature = "defmt"))]
        debug!("NtpPacket::new(tx_timestamp: {})", tx_timestamp);

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

    pub(crate) fn from_be_bytes(buf: &[u8; NTP_PACKET_LEN]) -> Self {
        // left it here for a while, maybe in future Rust releases there
        // will be a way to use such a generic function with compile-time
        // size determination
        // const fn to_array<T: Sized>(x: &[u8]) -> [u8; mem::size_of::<T>()] {
        //     let mut temp_buf = [0u8; mem::size_of::<T>()];
        //
        //     temp_buf.copy_from_slice(x);
        //     temp_buf
        // }
        // see: https://github.com/vpetrigo/sntpc/issues/34
        let to_array_u32 = |x: &[u8]| {
            let mut temp_buf = [0u8; size_of::<u32>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };
        let to_array_u64 = |x: &[u8]| {
            let mut temp_buf = [0u8; size_of::<u64>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };

        Self {
            li_vn_mode: buf[0],
            stratum: buf[1],
            #[allow(clippy::cast_possible_wrap)]
            poll: buf[2] as i8,
            #[allow(clippy::cast_possible_wrap)]
            precision: buf[3] as i8,
            root_delay: u32::from_be_bytes(to_array_u32(&buf[4..8])),
            root_dispersion: u32::from_be_bytes(to_array_u32(&buf[8..12])),
            ref_id: u32::from_be_bytes(to_array_u32(&buf[12..16])),
            ref_timestamp: u64::from_be_bytes(to_array_u64(&buf[16..24])),
            origin_timestamp: u64::from_be_bytes(to_array_u64(&buf[24..32])),
            recv_timestamp: u64::from_be_bytes(to_array_u64(&buf[32..40])),
            tx_timestamp: u64::from_be_bytes(to_array_u64(&buf[40..48])),
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

    /// Returns the precision of the timestamp generator as log2(seconds).
    /// For example, -20 means approximately 1 microsecond precision (2^-20 ≈ 0.954 µs).
    /// Default: -20 (typical for microsecond-precision clocks).
    fn precision(&self) -> i8 {
        -20
    }
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
            self.duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
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
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> impl Future<Output = Result<usize>>;

    /// Receives a single datagram message on the socket. On success, returns the number
    /// of bytes read and the origin.
    ///
    /// The function will be called with valid byte array `buf` of sufficient size to
    /// hold the message bytes
    /// # Errors
    ///
    /// Will return `Err` if an underlying UDP receive fails
    fn recv_from(&self, buf: &mut [u8]) -> impl Future<Output = Result<(usize, SocketAddr)>>;
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

#[derive(Copy, Clone)]
pub(crate) struct RawNtpPacket(pub(crate) [u8; NTP_PACKET_LEN]);

impl Default for RawNtpPacket {
    fn default() -> Self {
        RawNtpPacket([0u8; NTP_PACKET_LEN])
    }
}

impl From<&NtpPacket> for RawNtpPacket {
    #[allow(clippy::cast_sign_loss)]
    fn from(val: &NtpPacket) -> Self {
        let mut tmp_buf = [0u8; NTP_PACKET_LEN];

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
