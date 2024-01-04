use crate::types::{
    Error, NtpContext, NtpPacket, NtpResult, NtpTimestampGenerator,
    RawNtpPacket, Result, SendRequestResult,
};
use crate::{get_ntp_timestamp, process_response};
use core::fmt::Debug;
#[cfg(feature = "log")]
use log::debug;

#[cfg(feature = "std")]
use std::net::SocketAddr;
#[cfg(feature = "tokio")]
use tokio::net::{lookup_host, ToSocketAddrs};

#[cfg(not(feature = "std"))]
use no_std_net::{SocketAddr, ToSocketAddrs};

#[cfg(not(feature = "std"))]
async fn lookup_host<T>(host: T) -> Result<impl Iterator<Item = SocketAddr>>
where
    T: ToSocketAddrs,
{
    #[allow(unused_variables)]
    host.to_socket_addrs().map_err(|e| {
        #[cfg(feature = "log")]
        debug!("ToScoketAddrs: {}", e);
        Error::AddressResolve
    })
}

#[cfg(feature = "tokio")]
#[async_trait::async_trait]
pub trait NtpUdpSocket {
    async fn send_to<T: ToSocketAddrs + Send>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize>;

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
}

#[cfg(not(feature = "std"))]
pub trait NtpUdpSocket {
    fn send_to<T: ToSocketAddrs + Send>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> impl core::future::Future<Output = Result<usize>>;

    fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> impl core::future::Future<Output = Result<(usize, SocketAddr)>>;
}

pub async fn sntp_send_request<A, U, T>(
    dest: A,
    socket: &U,
    context: NtpContext<T>,
) -> Result<SendRequestResult>
where
    A: ToSocketAddrs + Debug + Send,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    #[cfg(feature = "log")]
    debug!("Address: {:?}, Socket: {:?}", dest, *socket);
    let request = NtpPacket::new(context.timestamp_gen);

    send_request(dest, &request, socket).await?;
    Ok(SendRequestResult::from(request))
}

async fn send_request<A: ToSocketAddrs + Send, U: NtpUdpSocket>(
    dest: A,
    req: &NtpPacket,
    socket: &U,
) -> core::result::Result<(), Error> {
    let buf = RawNtpPacket::from(req);

    match socket.send_to(&buf.0, dest).await {
        Ok(size) => {
            if size == buf.0.len() {
                Ok(())
            } else {
                Err(Error::Network)
            }
        }
        Err(_) => Err(Error::Network),
    }
}

pub async fn sntp_process_response<A, U, T>(
    dest: A,
    socket: &U,
    mut context: NtpContext<T>,
    send_req_result: SendRequestResult,
) -> Result<NtpResult>
where
    A: ToSocketAddrs + Debug,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    let mut response_buf = RawNtpPacket::default();
    let (response, src) = socket.recv_from(response_buf.0.as_mut()).await?;
    context.timestamp_gen.init();
    let recv_timestamp = get_ntp_timestamp(context.timestamp_gen);
    #[cfg(feature = "log")]
    debug!("Response: {}", response);

    match lookup_host(dest).await {
        Err(_) => return Err(Error::AddressResolve),
        Ok(mut it) => {
            if !it.any(|addr| addr == src) {
                return Err(Error::ResponseAddressMismatch);
            }
        }
    }

    if response != core::mem::size_of::<NtpPacket>() {
        return Err(Error::IncorrectPayload);
    }

    let result =
        process_response(send_req_result, response_buf, recv_timestamp);

    if let Ok(_r) = &result {
        #[cfg(feature = "log")]
        debug!("{:?}", _r);
    }

    result
}

pub async fn get_time<A, U, T>(
    pool_addrs: A,
    socket: U,
    context: NtpContext<T>,
) -> Result<NtpResult>
where
    A: ToSocketAddrs + Copy + Debug + Send,
    U: NtpUdpSocket + Debug,
    T: NtpTimestampGenerator + Copy,
{
    let result = sntp_send_request(pool_addrs, &socket, context).await?;

    sntp_process_response(pool_addrs, &socket, context, result).await
}
