use sntpc::{async_impl::get_time, NtpContext, StdTimestampGen};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

const POOL_NTP_ADDR: &str = "pool.ntp.org:123";

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap())
        .await
        .expect("Socket creation");
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    let res = get_time(POOL_NTP_ADDR, socket, ntp_context)
        .await
        .expect("get_time error");

    println!("RESULT: {res:?}");
}
