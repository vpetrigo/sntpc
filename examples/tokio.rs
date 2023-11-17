use sntpc::{
    async_impl::{get_time, NtpUdpSocket},
    Error, NtpContext, Result, StdTimestampGen,
};
use std::net::SocketAddr;
use tokio::net::{ToSocketAddrs, UdpSocket};

const POOL_NTP_ADDR: &str = "pool.ntp.org:123";

#[derive(Debug)]
struct Socket {
    sock: UdpSocket,
}

#[async_trait::async_trait]
impl NtpUdpSocket for Socket {
    async fn send_to<T: ToSocketAddrs + Send>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize> {
        self.sock
            .send_to(buf, addr)
            .await
            .map_err(|_| Error::Network)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.sock.recv_from(buf).await.map_err(|_| Error::Network)
    }
}

#[tokio::main]
async fn main() {
    let sock = UdpSocket::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap())
        .await
        .expect("Socket creation");
    let socket = Socket { sock: sock };
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    let res = get_time(POOL_NTP_ADDR, socket, ntp_context)
        .await
        .expect("get_time error");

    println!("RESULT: {:?}", res);
}
