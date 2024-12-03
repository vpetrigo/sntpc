use crate::types::{
    Error, NtpContext, NtpPacket, NtpResult, NtpTimestampGenerator,
    RawNtpPacket, Result, SendRequestResult,
};
use crate::{get_ntp_timestamp, process_response};
use core::fmt::Debug;
#[cfg(feature = "log")]
use log::debug;

use crate::net::SocketAddr;
#[cfg(not(feature = "std"))]
use crate::net::ToSocketAddrs;

#[cfg(feature = "tokio")]
use tokio::net::{lookup_host, ToSocketAddrs};

#[cfg(not(feature = "std"))]
#[allow(clippy::unused_async)]
async fn lookup_host<T>(host: T) -> Result<impl Iterator<Item = SocketAddr>>
where
    T: ToSocketAddrs,
{
    #[allow(unused_variables)]
    host.to_socket_addrs().map_err(|e| {
        #[cfg(feature = "log")]
        debug!("ToSocketAddrs: {:?}", e);
        Error::AddressResolve
    })
}

#[cfg(feature = "tokio")]
#[async_trait::async_trait]
pub trait NtpUdpSocket {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize>;

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
}

#[cfg(not(feature = "std"))]
pub trait NtpUdpSocket {
    fn send_to(
        &self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> impl core::future::Future<Output = Result<usize>>;

    fn recv_from(
        &self,
        buf: &mut [u8],
    ) -> impl core::future::Future<Output = Result<(usize, SocketAddr)>>;
}

impl NtpUdpSocket for &embassy_net::udp::UdpSocket<'_> {
    async fn send_to<T: ToSocketAddrs + Send>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize> {
        let saddr: SocketAddr = addr
            .to_socket_addrs()
            .map_err(|_| Error::AddressResolve)?
            .next()
            .ok_or(Error::AddressResolve)?;

        // Currently smoltcp still has its own address enum
        let endpoint = embassy_net::IpEndpoint::new(
            match saddr.ip() {
                crate::net::IpAddr::V4(addr) => {
                    embassy_net::IpAddress::Ipv4(addr)
                }
                crate::net::IpAddr::V6(addr) => {
                    embassy_net::IpAddress::Ipv6(addr)
                }
            },
            saddr.port(),
        );

        match embassy_net::udp::UdpSocket::send_to(self, buf, endpoint).await {
            Ok(_) => Ok(buf.len()),
            Err(e) => {
                #[cfg(feature = "log")]
                log::error!("Error while sending to {}: {:?}", endpoint, e);
                Err(Error::Network)
            }
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let to_addr = |ep: embassy_net::IpEndpoint| {
            SocketAddr::new(
                match ep.addr {
                    embassy_net::IpAddress::Ipv4(val) => {
                        crate::net::IpAddr::V4(val)
                    }
                    embassy_net::IpAddress::Ipv6(val) => {
                        crate::net::IpAddr::V6(val)
                    }
                },
                ep.port,
            )
        };

        match embassy_net::udp::UdpSocket::recv_from(self, buf).await {
            Ok((len, ep)) => Ok((len, to_addr(ep.endpoint))),
            Err(e) => {
                #[cfg(feature = "log")]
                log::error!("Error receiving {:?}", e);
                Err(Error::Network)
            }
        }
    }
}

/// # Errors
///
/// Will return `Err` if an SNTP request sending fails
pub async fn sntp_send_request<A, U, T>(
    dest: A,
    socket: &U,
    context: NtpContext<T>,
) -> Result<SendRequestResult>
where
    A: ToSocketAddrs + Debug + Send,
    U: NtpUdpSocket,
    T: NtpTimestampGenerator + Copy,
{
    #[cfg(feature = "log")]
    debug!("Address: {:?}", dest);
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

    let socket_addrs =
        lookup_host(dest).await.map_err(|_| Error::AddressResolve)?;

    // Try each available address.
    for addr in socket_addrs {
        if let Ok(size) = socket.send_to(&buf.0, addr).await {
            if size == buf.0.len() {
                return Ok(());
            }
        }
    }

    Err(Error::Network)
}

/// # Errors
///
/// Will return `Err` if an SNTP response processing fails
pub async fn sntp_process_response<A, U, T>(
    dest: A,
    socket: &U,
    mut context: NtpContext<T>,
    send_req_result: SendRequestResult,
) -> Result<NtpResult>
where
    A: ToSocketAddrs + Debug,
    U: NtpUdpSocket,
    T: NtpTimestampGenerator + Copy,
{
    let mut response_buf = RawNtpPacket::default();
    let (response, src) = socket.recv_from(response_buf.0.as_mut()).await?;
    context.timestamp_gen.init();
    let recv_timestamp = get_ntp_timestamp(&context.timestamp_gen);
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

    if response != size_of::<NtpPacket>() {
        return Err(Error::IncorrectPayload);
    }

    let result =
        process_response(send_req_result, response_buf, recv_timestamp);

    #[cfg(feature = "log")]
    if let Ok(r) = &result {
        debug!("{r:?}");
    }

    result
}

/// # Errors
///
/// Will return `Err` if an SNTP request cannot be sent or SNTP response fails
pub async fn get_time<A, U, T>(
    pool_addrs: A,
    socket: U,
    context: NtpContext<T>,
) -> Result<NtpResult>
where
    A: ToSocketAddrs + Copy + Debug + Send,
    U: NtpUdpSocket,
    T: NtpTimestampGenerator + Copy,
{
    let result = sntp_send_request(pool_addrs, &socket, context).await?;

    sntp_process_response(pool_addrs, &socket, context, result).await
}
