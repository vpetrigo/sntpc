//! Rust SNTP client
//!
//! This crate provides a method for sending requests to NTP servers
//! and process responses, extracting received timestamp
//!
//! # Example
//!
//! ```rust
//! use sntpc;
//!
//! let result = sntpc::request("pool.ntp.org", 123);
//!
//! if let Ok(timestamp) = result {
//!     println!("NTP server time: {}", timestamp);
//! }
//! ```

use std::io;
use std::mem;
use std::net;
use std::net::ToSocketAddrs;
use std::str;
use std::time;

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

impl NtpPacket {
    const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;
    #[allow(dead_code)]
    const LI_MASK: u8 = 0b0000_0011;
    #[allow(dead_code)]
    const VN_MASK: u8 = 0b0001_1100;
    #[allow(dead_code)]
    const MODE_MASK: u8 = 0b1110_0000;

    pub fn new() -> NtpPacket {
        let now_since_unix = time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .unwrap();
        let tx_timestamp = ((now_since_unix.as_secs()
            + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
            << 32)
            + u64::from(now_since_unix.subsec_micros());

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

/// Send request to a NTP server with the given address
/// and process the response
///
/// * `pool` - Server's name or IP address as a string
/// * `port` - Server's port as an int
///
/// # Example
///
/// ```rust
/// use sntpc;
///
/// let result = sntpc::request("time.google.com", 123);
/// // OR
/// let result = sntpc::request("83.168.200.199", 123);
///
/// // .. process the result
/// ```
pub fn request(pool: &str, port: u32) -> io::Result<u32> {
    dbg!(pool);
    let socket = net::UdpSocket::bind("0.0.0.0:0")
        .expect("Unable to create a UDP socket");
    let dest = format!("{}:{}", pool, port).to_socket_addrs()?;

    socket
        .set_read_timeout(Some(time::Duration::new(2, 0)))
        .expect("Unable to set up socket timeout");
    let req = NtpPacket::new();
    let mut success = false;

    for addr in dest {
        dbg!(&addr);

        match send_request(&req, &socket, addr) {
            Ok(write_bytes) => {
                assert_eq!(write_bytes, mem::size_of::<NtpPacket>());
                success = true;
                break;
            }
            Err(err) => {
                println!("{}", err);
                println!("Try another one");
                continue;
            }
        }
    }

    if !success {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to send NTP request",
        ));
    }

    let mut buf: RawNtpPacket = RawNtpPacket::default();
    let response = socket.recv_from(buf.0.as_mut())?;
    dbg!(response.0);

    if response.0 == mem::size_of::<NtpPacket>() {
        let result = process_response(buf);

        match result {
            Ok(timestamp) => return Ok(timestamp),
            Err(err_str) => {
                return Err(io::Error::new(io::ErrorKind::Other, err_str));
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Incorrect NTP packet size read",
    ))
}

fn send_request(
    req: &NtpPacket,
    socket: &net::UdpSocket,
    dest: net::SocketAddr,
) -> io::Result<usize> {
    let buf: RawNtpPacket = req.into();

    socket.send_to(&buf.0, dest)
}

fn process_response(resp: RawNtpPacket) -> Result<u32, &'static str> {
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);

    if packet.li_vn_mode == 0 || packet.stratum == 0 {
        return Err("Incorrect LI_VN_MODE or STRATUM headers");
    }

    if packet.origin_timestamp == 0 || packet.recv_timestamp == 0 {
        return Err("Invalid origin/receive timestamp");
    }

    if packet.tx_timestamp == 0 {
        return Err("Transmit timestamp is 0");
    }

    #[cfg(debug_assertions)]
    debug_ntp_packet(&packet);

    let seconds = (packet.tx_timestamp >> 32) as u32;
    let tx_tm = seconds - NtpPacket::NTP_TIMESTAMP_DELTA;

    Ok(tx_tm)
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
    const MODE_MASK: u8 = 0b0000_0111;
    const MODE_SHIFT: u8 = 0;
    const VERSION_MASK: u8 = 0b0011_1000;
    const VERSION_SHIFT: u8 = 3;
    const LI_MASK: u8 = 0b1100_0000;
    const LI_SHIFT: u8 = 6;

    let shifter = |val, mask, shift| (val & mask) >> shift;
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);

    println!("{}", (0..52).map(|_| "=").collect::<String>());
    println!("| Mode:\t\t{}", mode);
    println!("| Version:\t{}", version);
    println!("| Leap:\t\t{}", li);
    println!("| Poll:\t\t{}", packet.poll);
    println!("| Precision:\t\t{}", packet.precision);
    println!("| Root delay:\t\t{}", packet.root_delay);
    println!("| Root dispersion:\t{}", packet.root_dispersion);
    println!(
        "| Reference ID:\t\t{}",
        str::from_utf8(&packet.ref_id.to_be_bytes()).unwrap_or("")
    );
    println!("| Reference timestamp:\t{:>16}", packet.ref_timestamp);
    println!("| Origin timestamp:\t\t{:>16}", packet.origin_timestamp);
    println!("| Receive timestamp:\t{:>16}", packet.recv_timestamp);
    println!("| Transmit timestamp:\t{:>16}", packet.tx_timestamp);
    println!("{}", (0..52).map(|_| "=").collect::<String>());
}
