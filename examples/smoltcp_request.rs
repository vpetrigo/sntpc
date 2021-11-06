use core::cell::RefCell;
use core::default::Default;
use core::fmt::Debug;

use std::collections::BTreeMap;
use std::fmt::Formatter;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
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
        f.debug_struct("SmoltcpUdpSocketWrapper").finish()
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

            println!("{}", endpoint);

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

    const APP_PORT: u16 = 6666;
    let interface_name = "tap0";
    let tuntap =
        TapInterface::new(interface_name).expect("Cannot create TAP interface");
    let to_port = 123;

    let mut buffer = Buffers::default();
    let udp_buffer = UdpSocketBuffers::new(&mut buffer);

    let socket = UdpSocket::new(udp_buffer.rx, udp_buffer.tx);
    // TODO: Add support for setting ethernet MAC, IP address and gateway addresses via env/CLI
    let ethernet_addr = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);
    let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24)];
    let default_v4_gw = Ipv4Address::new(192, 168, 69, 100);
    let mut routes_storage = [None; 3];
    let mut routes = Routes::new(&mut routes_storage[..]);
    routes.add_default_ipv4_route(default_v4_gw).unwrap();
    // routes.add_default_ipv4_route(default_v4_gw2).unwrap();
    // routes.add_default_ipv4_route(default_v4_gw3).unwrap();
    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let mut iface = EthernetInterfaceBuilder::new(tuntap)
        .ethernet_addr(ethernet_addr)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut socket_items = [None; 1];
    let mut sockets = SocketSet::new(socket_items.as_mut());

    let udp_handle = sockets.add(socket);
    let mut counter = 0;
    let mut once = true;

    loop {
        let timestamp = Instant::now();

        match iface.poll(&mut sockets, timestamp) {
            Ok(_) => println!("Poll ok!"),
            Err(e) => println!("Poll error: {}!", e),
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        {
            {
                let mut socket = sockets.get::<UdpSocket>(udp_handle);
                if !socket.is_open() {
                    socket.bind(APP_PORT).unwrap();
                }
            }

            // let ep =
            //     IpEndpoint::from((IpAddress::v4(192, 168, 69, 100), to_port));
            // let to_send = format!("Hello {}\n", counter);
            if once {
                // once = false;
                let sock_wrapper = SmoltcpUdpSocketWrapper {
                    socket: RefCell::new(sockets.get::<UdpSocket>(udp_handle)),
                };
                let context = NtpContext::new(StdTimestampGen::default());
                let result = sntpc::request_with_addrs(
                    SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(216, 239, 35, 8)),
                        to_port,
                    ),
                    sock_wrapper,
                    context,
                );

                println!("{:?}", result);
            } else {
                let mut socket = sockets.get::<UdpSocket>(udp_handle);
                let mut buf = [0u8; 48];
                socket.recv_slice(&mut buf);

                println!("{:?}", buf);
            }

            counter += 1;
            // socket.send_slice(to_send.as_bytes(), ep).unwrap();
        }

        wait(
            iface.device().as_raw_fd(),
            iface.poll_delay(&sockets, Instant::from_secs(5)),
        )
        .unwrap();
    }
}
