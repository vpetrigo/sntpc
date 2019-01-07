pub mod sntp {
    pub const SNTP_CLIENT: u8 = 4;
    pub const LI_MASK: u8 = 0b0000_0011;
    pub const VN_MASK: u8 = 0b0001_1100;
    pub const MODE_MASK: u8 = 0b1110_0000;

    pub struct NtpPacket {
        li_vn_mode: u8,
        stratum: u8,
        poll: u8,
        precision: u8,
        root_delay: u32,
        root_dispersion: u32,
        ref_id: u32,
        ref_timestamp: u64,
        origin_timestamp: u64,
        recv_timestamp: u64,
        tx_timestamp: u64,
    }
}
