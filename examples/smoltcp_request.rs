use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::wait;
use smoltcp::phy::Device;
use smoltcp::phy::TapInterface;
use smoltcp::socket::{SocketSet, UdpSocket, UdpSocketBuffer};
use smoltcp::storage::PacketMetadata;
use smoltcp::time::Instant;
use smoltcp::wire::{
    EthernetAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address,
};
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use std::os::unix::prelude::AsRawFd;

fn main() {
    const APP_PORT: u16 = 6666;
    let interface_name = "tap0";
    let tuntap =
        TapInterface::new(interface_name).expect("Cannot create TAP interface");
    let to_address = "192.168.69.1";
    let to_port = 123;
    let mut rx_meta = [PacketMetadata::<IpEndpoint>::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::<IpEndpoint>::EMPTY; 4];
    let mut rx_buffer = [0u8; 256];
    let mut tx_buffer = [0u8; 64];
    let rx_sock_buffer =
        UdpSocketBuffer::new(rx_meta.as_mut(), rx_buffer.as_mut());
    let tx_sock_buffer =
        UdpSocketBuffer::new(tx_meta.as_mut(), tx_buffer.as_mut());
    let mut socket = UdpSocket::new(rx_sock_buffer, tx_sock_buffer);
    let ethernet_addr = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);
    let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 69, 1), 24)];
    let default_v4_gw = Ipv4Address::new(192, 168, 69, 100);
    let mut routes_storage = [None; 2];
    let mut routes = Routes::new(&mut routes_storage[..]);
    routes.add_default_ipv4_route(default_v4_gw).unwrap();
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

    loop {
        let timestamp = Instant::now();

        match iface.poll(&mut sockets, timestamp) {
            Ok(_) => println!("Poll ok!"),
            Err(e) => println!("Poll error: {}!", e),
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        {
            let mut socket = sockets.get::<UdpSocket>(udp_handle);
            if !socket.is_open() {
                socket.bind(APP_PORT).unwrap();
            }

            let ep =
                IpEndpoint::from((IpAddress::v4(192, 168, 69, 100), to_port));
            let to_send = format!("Hello {}\n", counter);

            counter += 1;
            socket.send_slice(to_send.as_bytes(), ep).unwrap();
        }

        wait(
            iface.device().as_raw_fd(),
            iface.poll_delay(&sockets, Instant::from_secs(5)),
        ).unwrap();
    }
}
