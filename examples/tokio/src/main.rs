use sntpc::{
    get_time, Error, NtpContext, NtpUdpSocket, Result, StdTimestampGen,
};
use tokio::net::{lookup_host, UdpSocket};

use core::net::SocketAddr;

const POOL_NTP_ADDR: (&str, u16) = ("pool.ntp.org", 123);

#[derive(Debug)]
struct Socket {
    sock: UdpSocket,
}

impl NtpUdpSocket for Socket {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        self.sock
            .send_to(buf, addr)
            .await
            .map_err(|_| Error::Network)
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.sock.recv_from(buf).await.map_err(|_| Error::Network)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let sock = UdpSocket::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap())
        .await
        .expect("Socket creation");
    let socket = Socket { sock };
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    for addr in lookup_host(POOL_NTP_ADDR)
        .await
        .expect("Unable to resolve address")
    {
        let res = get_time(addr, &socket, ntp_context)
            .await
            .expect("get_time error");

        println!("RESULT: {res:?}");
    }
}
