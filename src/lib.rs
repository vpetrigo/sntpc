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

    pub fn create_client_req() -> NtpPacket {
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
            tx_timestamp: 0,
        }
    }

    pub fn request(pool: &str, port: u32) {
        dbg!(pool);
        let socket = net::UdpSocket::bind("0.0.0.0:0")
            .expect("Unable to create a UDP socket");
        let dest = format!("{}:{}", pool, port);

        socket
            .set_read_timeout(Some(time::Duration::new(2, 0)))
            .expect("Unable to set up socket timeout");
        let req = create_client_req();
        unsafe {
            let buf: *const [u8] = slice::from_raw_parts(
                (&req as *const NtpPacket) as *const u8,
                mem::size_of::<NtpPacket>(),
            );
            let res = socket
                .send_to(buf.as_ref().unwrap(), dest)
                .expect("Unable to send NTP request");
            println!("Send: {}", res);
        }

        let mut buf: [u8; 48] = [0u8; 48];
        let response = socket
            .recv_from(buf.as_mut())
            .ok();

        if let Some(m) = response {
            println!("Read bytes: {}", m.0);
            for i in buf.iter() {
                print!("[{}]", i);
            }
            println!();

            let packet = unsafe { mem::transmute::<[u8; 48], NtpPacket>(buf) };
            let mut tmp_buf: [u8; 4] = [0; 4];

            tmp_buf.copy_from_slice(&buf[32..36]);
            let tx_tm = unsafe { u32::from_be_bytes(tmp_buf) };
            println!("{}", tx_tm - NTP_TIMESTAMP_DELTA);
        }
    }
}

pub use sntp::*;
