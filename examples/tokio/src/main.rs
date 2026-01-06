use sntpc::{get_time, NtpContext, StdTimestampGen};
use sntpc_net_tokio::UdpSocketWrapper;
use tokio::net::{lookup_host, UdpSocket};
use tokio::time::timeout;

use core::net::SocketAddr;

const POOL_NTP_ADDR: (&str, u16) = ("pool.ntp.org", 123);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket = UdpSocket::bind("0.0.0.0:0".parse::<SocketAddr>().unwrap())
        .await
        .expect("Socket creation");
    let socket = UdpSocketWrapper::from(socket);
    let ntp_context = NtpContext::new(StdTimestampGen::default());

    for addr in lookup_host(POOL_NTP_ADDR).await.expect("Unable to resolve address") {
        let duration = core::time::Duration::from_secs(2);
        let res = timeout(duration, get_time(addr, &socket, ntp_context)).await;

        match res {
            Ok(Ok(res)) => println!("RESULT: {res:?}"),
            Ok(Err(err)) => println!("ERROR: {err:?}"),
            Err(_) => println!("TIMEOUT: address {addr:?}"),
        }
    }
}
