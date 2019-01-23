pub mod sntp {
    use std::io;
    use std::mem;
    use std::net;
    use std::net::ToSocketAddrs;
    use std::slice;
    use std::str;
    use std::time;

    pub const NTP_TIMESTAMP_DELTA: u32 = 2208988800u32;
    pub const SNTP_CLIENT_MODE: u8 = 3;
    pub const SNTP_VERSION: u8 = 4 << 3;
    pub const LI_MASK: u8 = 0b0000_0011;
    pub const VN_MASK: u8 = 0b0001_1100;
    pub const MODE_MASK: u8 = 0b1110_0000;

    /// ```
    /// use std::mem;
    /// use sntp_client::NtpPacket;
    ///
    /// assert_eq!(mem::size_of::<NtpPacket>(), 48);
    /// ```
    #[repr(packed)]
    pub struct NtpPacket {
        li_vn_mode: u8,
        stratum: u8,
        #[allow(dead_code)]
        poll: i8,
        #[allow(dead_code)]
        precision: i8,
        root_delay: u32,
        root_dispersion: u32,
        ref_id: u32,
        ref_timestamp: u64,
        origin_timestamp: u64,
        recv_timestamp: u64,
        tx_timestamp: u64,
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

    pub fn create_client_req() -> NtpPacket {
        let now_since_unix = time::SystemTime::now()
            .duration_since(time::SystemTime::UNIX_EPOCH)
            .unwrap();
        let tx_timestamp =
            (((now_since_unix.as_secs() + NTP_TIMESTAMP_DELTA as u64) << 32)
                + now_since_unix.subsec_micros() as u64)
                .to_be();

        NtpPacket {
            li_vn_mode: SNTP_CLIENT_MODE | SNTP_VERSION,
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

    pub fn request(pool: &str, port: u32) -> io::Result<u32> {
        dbg!(pool);
        let socket = net::UdpSocket::bind("0.0.0.0:0")
            .expect("Unable to create a UDP socket");
        let dest = format!("{}:{}", pool, port).to_socket_addrs()?;

        socket
            .set_read_timeout(Some(time::Duration::new(2, 0)))
            .expect("Unable to set up socket timeout");
        let req = create_client_req();
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
                    dbg!(err);
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

        let mut buf: [u8; mem::size_of::<NtpPacket>()] =
            [0; mem::size_of::<NtpPacket>()];
        let response = socket.recv_from(buf.as_mut())?;
        dbg!(response.0);

        if response.0 == mem::size_of::<NtpPacket>() {
            return Ok(process_response(&buf).unwrap_or(0));
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
        unsafe {
            let buf: *const [u8] = slice::from_raw_parts(
                (req as *const NtpPacket) as *const u8,
                mem::size_of::<NtpPacket>(),
            );
            let res = socket.send_to(buf.as_ref().unwrap(), dest)?;

            Ok(res)
        }
    }

    fn process_response(
        resp: &[u8; mem::size_of::<NtpPacket>()],
    ) -> Result<u32, &str> {
        let mut packet = unsafe {
            mem::transmute::<[u8; mem::size_of::<NtpPacket>()], NtpPacket>(
                *resp,
            )
        };

        dbg!(packet.origin_timestamp);
        dbg!(packet.recv_timestamp);
        convert_from_network(&mut packet);

        if packet.li_vn_mode == 0 || packet.stratum == 0 {
            return Err("Incorrect LI_VN_MODE or STRATUM headers");
        }

        #[cfg(debug_assertions)]
        debug_ntp_packet(&packet);

        let seconds = (packet.tx_timestamp >> 32) as u32;
        let tx_tm = seconds - NTP_TIMESTAMP_DELTA;

        return Ok(tx_tm);
    }

    fn convert_from_network(packet: &mut NtpPacket) {
        fn ntohl<T: NtpNum<T>>(val: T) -> T { val.ntohl() }

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

        unsafe {
            println!("Mode:\t\t{}", mode);
            println!("Version:\t{}", version);
            println!("Leap:\t\t{}", li);
            println!("Poll:\t\t{}", packet.poll);
            println!("Precision:\t\t{}", packet.precision);
            println!("Root delay:\t\t{}", packet.root_delay);
            println!("Root dispersion:\t{}", packet.root_dispersion);
            println!(
                "Reference ID:\t\t{}",
                str::from_utf8(&packet.ref_id.to_be_bytes()).unwrap_or("")
            );
            println!("Reference timestamp:\t{}", packet.ref_timestamp);
            println!("Origin timestamp:\t\t{}", packet.origin_timestamp);
            println!("Receive timestamp:\t\t{}", packet.recv_timestamp);
            println!("Transmit timestamp:\t\t{}", packet.tx_timestamp);
        }
    }
}

pub use sntp::*;
