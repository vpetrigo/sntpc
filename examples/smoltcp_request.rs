use core::cell::RefCell;
use core::default::Default;
use core::fmt::Debug;
use core::str::FromStr;

use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::os::unix::prelude::AsRawFd;

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::wait;
use smoltcp::phy::TapInterface;
use smoltcp::socket::{SocketRef, SocketSet, UdpSocket, UdpSocketBuffer};
use smoltcp::storage::PacketMetadata;
use smoltcp::time::Instant;
use smoltcp::wire::{
    EthernetAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address,
};

use clap::{crate_version, App, Arg};

use sntpc::{self, Error, NtpContext, NtpTimestamp, NtpUdpSocket};

#[cfg(feature = "log")]
use log;
#[cfg(feature = "log")]
use simple_logger;

struct Buffers {
    rx_meta: [PacketMetadata<IpEndpoint>; 16],
    tx_meta: [PacketMetadata<IpEndpoint>; 16],
    rx_buffer: [u8; 256],
    tx_buffer: [u8; 256],
}

impl Default for Buffers {
    fn default() -> Self {
        Buffers {
            rx_meta: [PacketMetadata::<IpEndpoint>::EMPTY; 16],
            tx_meta: [PacketMetadata::<IpEndpoint>::EMPTY; 16],
            rx_buffer: [0u8; 256],
            tx_buffer: [0u8; 256],
        }
    }
}

struct UdpSocketBuffers<'a> {
    rx: UdpSocketBuffer<'a>,
    tx: UdpSocketBuffer<'a>,
}

impl<'a> UdpSocketBuffers<'a> {
    fn new(buffers: &'a mut Buffers) -> Self {
        UdpSocketBuffers {
            rx: UdpSocketBuffer::new(
                buffers.rx_meta.as_mut(),
                buffers.rx_buffer.as_mut(),
            ),
            tx: UdpSocketBuffer::new(
                buffers.tx_meta.as_mut(),
                buffers.tx_buffer.as_mut(),
            ),
        }
    }
}

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: std::time::Duration,
}

impl NtpTimestamp for StdTimestampGen {
    fn init(&mut self) {
        self.duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

struct SmoltcpUdpSocketWrapper<'a, 'b> {
    socket: RefCell<SocketRef<'b, UdpSocket<'a>>>,
}

impl<'a, 'b> Debug for SmoltcpUdpSocketWrapper<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmoltcpUdpSocketWrapper")
            .field("socket", &self.socket.borrow().endpoint())
            .finish()
    }
}

impl<'a, 'b> NtpUdpSocket for SmoltcpUdpSocketWrapper<'a, 'b> {
    fn send_to<T: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize, Error> {
        if let Ok(mut iter) = addr.to_socket_addrs() {
            let addr = if let Some(sock_addr) = iter.next() {
                sock_addr
            } else {
                return Err(Error::Network);
            };

            let endpoint = match addr {
                SocketAddr::V4(v4) => IpEndpoint::from(v4),
                SocketAddr::V6(_) => return Err(Error::Network),
            };

            if let Ok(_) =
                self.socket.borrow_mut().send_slice(&buf[..], endpoint)
            {
                return Ok(buf.len());
            }
        }

        Err(Error::Network)
    }

    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
        let result = self.socket.borrow_mut().recv_slice(&mut buf[..]);

        if let Ok((size, address)) = result {
            let sockaddr = match address.addr {
                IpAddress::Ipv4(v4) => SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(
                        v4.0[0], v4.0[1], v4.0[2], v4.0[3],
                    )),
                    address.port,
                ),
                _ => return Err(Error::Network),
            };

            return Ok((size, sockaddr));
        }

        Err(Error::Network)
    }
}

fn main() {
    #[cfg(feature = "log")]
    if cfg!(feature = "log") {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    }

    const GOOGLE_NTP_ADDR: &str = "time.google.com";
    const APP_PORT: &str = "6666";

    let app = App::new("smoltcp_request")
        .version(crate_version!())
        .arg(
            Arg::with_name("server")
                .short("s")
                .long("server")
                .takes_value(true)
                .default_value(GOOGLE_NTP_ADDR)
                .help("NTP server hostname"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .default_value("123")
                .help("NTP server port"),
        )
        .arg(
            Arg::with_name("interface")
                .short("i")
                .long("interface")
                .required(true)
                .takes_value(true)
                .help("Ethernet interface smoltcp to bind to"),
        )
        .arg(
            Arg::with_name("mac")
                .short("m")
                .long("mac")
                .default_value("02:00:00:00:00:02")
                .takes_value(true)
                .help("Device MAC address"),
        )
        .arg(
            Arg::with_name("ip")
                .long("ip")
                .takes_value(true)
                .required(true)
                .help("Device IP address assigned with the interface"),
        )
        .arg(
            Arg::with_name("gw")
                .long("gw")
                .takes_value(true)
                .required(true)
                .help("Device default gateway"),
        )
        .arg(
            Arg::with_name("sock_port")
                .long("sock_port")
                .takes_value(true)
                .default_value(APP_PORT)
                .help("Device port to bind UDP socket to"),
        )
        .get_matches();

    let interface_name = app.value_of("interface").unwrap();
    let tuntap =
        TapInterface::new(interface_name).expect("Cannot create TAP interface");

    let server_ip = app.value_of("server").unwrap();
    let server_port = u16::from_str(app.value_of("port").unwrap())
        .expect("Unable to parse server port");
    let server_sock_addr =
        SocketAddr::new(IpAddr::from_str(server_ip).unwrap(), server_port);
    let eth_address = EthernetAddress::from_str(app.value_of("mac").unwrap())
        .expect("Cannot parse MAC address of the interface");
    let ip_addr = IpAddress::from_str(app.value_of("ip").unwrap())
        .expect("Cannot parse IP address of the interface");
    let default_gw = Ipv4Address::from_str(app.value_of("gw").unwrap())
        .expect("Cannot parse GW address of the interface");
    let sock_port = u16::from_str(app.value_of("sock_port").unwrap())
        .expect("Unable to parse socket port");

    let mut buffer = Buffers::default();
    let udp_buffer = UdpSocketBuffers::new(&mut buffer);

    let mut socket = UdpSocket::new(udp_buffer.rx, udp_buffer.tx);
    socket.bind(sock_port).unwrap();
    let ip_addrs = [IpCidr::new(ip_addr, 24)];
    let mut routes_storage = [None; 3];
    let mut routes = Routes::new(&mut routes_storage[..]);
    routes.add_default_ipv4_route(default_gw.into()).unwrap();
    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let mut iface = EthernetInterfaceBuilder::new(tuntap)
        .ethernet_addr(eth_address)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut socket_items = [None; 1];
    let mut sockets = SocketSet::new(socket_items.as_mut());

    let udp_handle = sockets.add(socket);
    let mut once_tx = true;
    let mut once_rx = true;
    let mut send_result = Option::None;

    while once_rx {
        let timestamp = Instant::now();

        match iface.poll(&mut sockets, timestamp) {
            Ok(_) => {
                #[cfg(feature = "log")]
                log::trace!("Poll ok!");
            }
            Err(_e) => {
                #[cfg(feature = "log")]
                log::trace!("Poll error: {}!", _e);
            }
        }

        {
            if once_tx && sockets.get::<UdpSocket>(udp_handle).can_send() {
                once_tx = false;
                let sock_wrapper = SmoltcpUdpSocketWrapper {
                    socket: RefCell::new(sockets.get::<UdpSocket>(udp_handle)),
                };
                let context = NtpContext::new(StdTimestampGen::default());
                let result = sntpc::sntp_send_request(
                    server_sock_addr,
                    &sock_wrapper,
                    context,
                );

                match result {
                    Ok(result) => {
                        send_result = Some(result);
                    }
                    Err(_e) => {
                        #[cfg(feature = "log")]
                        log::error!("send error: {:?}", _e);
                        once_tx = true;
                    }
                }

                #[cfg(feature = "log")]
                log::trace!("{:?}", &result);
            }

            if once_rx
                && sockets.get::<UdpSocket>(udp_handle).can_recv()
                && send_result.is_some()
            {
                once_rx = false;

                let sock_wrapper = SmoltcpUdpSocketWrapper {
                    socket: RefCell::new(sockets.get::<UdpSocket>(udp_handle)),
                };
                let context = NtpContext::new(StdTimestampGen::default());

                let _result = sntpc::sntp_process_response(
                    server_sock_addr,
                    &sock_wrapper,
                    context,
                    send_result.unwrap(),
                );

                #[cfg(feature = "log")]
                log::info!("{:?}", _result);
            }
        }

        wait(
            iface.device().as_raw_fd(),
            iface.poll_delay(&sockets, Instant::from_secs(1)),
        )
        .unwrap();
    }
}
