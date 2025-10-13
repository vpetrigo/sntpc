//! Demonstrates how to use [`smoltcp`](https://github.com/smoltcp-rs/smoltcp) stack with the
//! [`sntpc`] library
//!
//! Unfortunately, some `std` requirements still imposed due to TAP interface creation is dependent
//! on UNIX OS specific calls in the standard library. This example should provide all details on
//! how to set up networking interface to use with the `sntpc` library though.
//!
//! ## How to set up the environment (IPv4 only considered for now):
//!
//! - create TAP interface (`sudo` may require):
//! ```sh
//! $ ip tuntap add name tap0 mode tap
//! $ ip link set tap0 up
//! $ ip addr add 192.168.69.1/24 dev tap0
//! ```
//! - check that forwarding is enabled in the system:
//! ```sh
//! $ sysctl net.ipv4.ip_forward
//! # net.ipv4.ip_forward = 1
//! # if net.ipv4.ip_forward = 0 execute:
//! $ sysctl net.ipv4.ip_forward=1
//! ```
//! - enable forwarding and masquerading to allow internet access for the example app:
//! ```sh
//! # Fedora firewalld (initial state)
//! $ firewall-cmd --list-all
//! FedoraWorkstation (active)
//!   interfaces: ens33
//!   forward: no
//!   masquerade: yes
//! # add tap0 interface to the active firewalld zone
//! $ firewall-cmd --zone=FedoraWorkstation --add-interface=tap0
//! $ firewall-cmd --list-all
//! firewall-cmd --list-all
//! FedoraWorkstation (active)
//!   interfaces: ens33 tap0 <--- !
//!   forward: no
//!   masquerade: yes
//! # enable masquerade and forward
//! $ firewall-cmd --zone=FedoraWorkstation --add-masquerade
//! $ firewall-cmd --zone=FedoraWorkstation --add-forward
//! $ firewall-cmd --list-all
//! FedoraWorkstation (active)
//!   interfaces: ens33 tap0
//!   forward: yes <--- !
//!   masquerade: yes <--- !
//! ```
//! That is, you runtime firewalld setup should allow the example app to get access to internet
//! hosts. In order to preserve that settings permanents you may execute the following command:
//! ```sh
//! $ firewall-cmd --runtime-to-permanent
//! ```
//! So that, all firewalld configs will be preserved between reboots.
//!
//! ## How to run the example app:
//!
//! This example uses [`clap`](https://crates.io/crates/clap) to process command line arguments.
//! Currently, the following options are available:
//! ```sh
//! OPTIONS:
//!         --gw <gw>                  Device default gateway
//!     -i, --interface <interface>    Ethernet interface smoltcp to bind to
//!         --ip <ip>                  Device IP address assigned with the interface in the format <IP>/<Subnet Mask>
//!     -m, --mac <mac>                Device MAC address [default: 02:00:00:00:00:02]
//!     -p, --port <port>              NTP server port [default: 123]
//!     -s, --server <server>          NTP server hostname [default: time.google.com]
//!         --sock_port <sock_port>    Device port to bind UDP socket to [default: 6666]
//! ```
//!
//! Ready-to-use command line that reflects network interface setup mentioned above:
//! ```sh
//! $ cargo run --package sntpc --example smoltcp_request --no-default-features --features "std log" -- --server "216.239.35.12" --port "123" -i "tap0" -m "02:00:00:00:00:02" --ip "192.168.69.2/24" --gw "192.168.69.1"
//! ```
//!
//! As a result you should see something like that at the end of log output:
//! ```
//! $ 2021-11-08 23:53:29,950 INFO [smoltcp_request] Ok(NtpResult { seconds: 1636404809, seconds_fraction: 4004704152, roundtrip: 36149, offset: 927 })
//! ```
//!
#[cfg(unix)]
use {
    core::cell::RefCell,
    core::net::{IpAddr, SocketAddr},
    core::str::FromStr,
    smoltcp::iface::PollResult,
    smoltcp::iface::{Config, Interface, SocketSet},
    smoltcp::phy::TunTapInterface,
    smoltcp::phy::{wait, Medium},
    smoltcp::socket::udp,
    smoltcp::time::Instant,
    smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address},
    sntpc::{
        sync::{sntp_process_response, sntp_send_request},
        NtpContext,
    },
    std::os::unix::prelude::AsRawFd,
};

#[cfg(unix)]
pub mod internal {
    use {
        clap::{crate_version, App, Arg, ArgMatches},
        core::cell::RefCell,
        core::fmt::Debug,
        smoltcp::socket::udp,
        smoltcp::socket::udp::UdpMetadata,
        smoltcp::storage::PacketMetadata,
        smoltcp::wire::{IpAddress, IpEndpoint},
        sntpc::{Error, NtpTimestampGenerator, NtpUdpSocket},
        std::fmt::Formatter,
        std::net::{IpAddr, SocketAddr},
    };
    pub struct Buffers {
        pub rx_meta: [PacketMetadata<UdpMetadata>; 16],
        pub tx_meta: [PacketMetadata<UdpMetadata>; 16],
        pub rx_buffer: [u8; 256],
        pub tx_buffer: [u8; 256],
    }

    impl Default for Buffers {
        fn default() -> Self {
            Buffers {
                rx_meta: [PacketMetadata::EMPTY; 16],
                tx_meta: [PacketMetadata::EMPTY; 16],
                rx_buffer: [0u8; 256],
                tx_buffer: [0u8; 256],
            }
        }
    }

    pub struct UdpSocketBuffers<'a> {
        pub rx: udp::PacketBuffer<'a>,
        pub tx: udp::PacketBuffer<'a>,
    }

    impl<'a> UdpSocketBuffers<'a> {
        pub fn new(buffers: &'a mut Buffers) -> Self {
            UdpSocketBuffers {
                rx: udp::PacketBuffer::new(buffers.rx_meta.as_mut(), buffers.rx_buffer.as_mut()),
                tx: udp::PacketBuffer::new(buffers.tx_meta.as_mut(), buffers.tx_buffer.as_mut()),
            }
        }
    }

    #[derive(Copy, Clone, Default)]
    pub struct StdTimestampGen {
        duration: std::time::Duration,
    }

    impl NtpTimestampGenerator for StdTimestampGen {
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

    pub struct SmoltcpUdpSocketWrapper<'a, 'b> {
        pub socket: RefCell<&'b mut udp::Socket<'a>>,
    }

    impl Debug for SmoltcpUdpSocketWrapper<'_, '_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SmoltcpUdpSocketWrapper")
                .field("socket", &self.socket.borrow().endpoint())
                .finish()
        }
    }

    impl NtpUdpSocket for SmoltcpUdpSocketWrapper<'_, '_> {
        async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize, Error> {
            let endpoint = match addr {
                SocketAddr::V4(v4) => IpEndpoint::from(v4),
                SocketAddr::V6(_) => return Err(Error::Network),
            };

            if self.socket.borrow_mut().send_slice(buf, endpoint).is_ok() {
                return Ok(buf.len());
            }

            Err(Error::Network)
        }

        async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), Error> {
            let result = self.socket.borrow_mut().recv_slice(&mut buf[..]);

            if let Ok((size, address)) = result {
                // make compiler and clippy happy as without the else branch clippy complains
                // that not all variants covered for some reason
                #[allow(irrefutable_let_patterns)]
                let IpAddress::Ipv4(v4) = address.endpoint.addr
                else {
                    todo!()
                };
                let sockaddr = SocketAddr::new(IpAddr::V4(v4), address.endpoint.port);

                return Ok((size, sockaddr));
            }

            Err(Error::Network)
        }
    }

    #[must_use]
    pub fn create_app_cli() -> ArgMatches<'static> {
        const GOOGLE_NTP_ADDR: &str = "pool.ntp.org";
        const APP_PORT: &str = "6666";

        App::new("smoltcp_request")
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
                    .help("Device IP address assigned with the interface in the format <IP>/<Subnet Mask>"),
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
            .get_matches()
    }
}

#[cfg(unix)]
use internal::{create_app_cli, Buffers, SmoltcpUdpSocketWrapper, StdTimestampGen, UdpSocketBuffers};

#[cfg(unix)]
fn main() {
    #[cfg(feature = "log")]
    if cfg!(feature = "log") {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    }

    let app = create_app_cli();
    let interface_name = app.value_of("interface").unwrap();
    let mut tuntap = TunTapInterface::new(interface_name, Medium::Ethernet).expect("Cannot create TAP interface");

    let server_ip = app.value_of("server").unwrap();
    let server_port = u16::from_str(app.value_of("port").unwrap()).expect("Unable to parse server port");
    let server_sock_addr = SocketAddr::new(IpAddr::from_str(server_ip).unwrap(), server_port);
    let eth_address =
        EthernetAddress::from_str(app.value_of("mac").unwrap()).expect("Cannot parse MAC address of the interface");
    let ip_addr = IpCidr::from_str(app.value_of("ip").unwrap()).expect("Cannot parse IP address of the interface");
    let default_gw =
        Ipv4Address::from_str(app.value_of("gw").unwrap()).expect("Cannot parse GW address of the interface");
    let sock_port = u16::from_str(app.value_of("sock_port").unwrap()).expect("Unable to parse socket port");

    let mut buffer = Buffers::default();
    let udp_buffer = UdpSocketBuffers::new(&mut buffer);

    let mut socket = udp::Socket::new(udp_buffer.rx, udp_buffer.tx);
    socket.bind(sock_port).unwrap();
    let mut config = Config::new(eth_address.into());

    config.random_seed = 0;

    let mut iface = Interface::new(config, &mut tuntap, std::time::Instant::now().into());
    iface.update_ip_addrs(|ip_addrs| ip_addrs.push(ip_addr).unwrap());
    iface.routes_mut().add_default_ipv4_route(default_gw).unwrap();

    // let mut socket_items = [None; 1];
    let mut sockets = SocketSet::new(vec![]);
    let udp_handle = sockets.add(socket);
    let mut once_tx = true;
    let mut once_rx = true;
    let mut send_result = None;

    while once_rx {
        let timestamp = Instant::now();

        if matches!(
            iface.poll(timestamp, &mut tuntap, &mut sockets),
            PollResult::SocketStateChanged
        ) {
            #[cfg(feature = "log")]
            log::trace!("Poll ok!");
        }

        if once_tx && sockets.get::<udp::Socket>(udp_handle).can_send() {
            once_tx = false;
            let sock_wrapper = SmoltcpUdpSocketWrapper {
                socket: RefCell::new(sockets.get_mut::<udp::Socket>(udp_handle)),
            };
            let context = NtpContext::new(StdTimestampGen::default());
            let result = sntp_send_request(server_sock_addr, &sock_wrapper, context);

            match result {
                Ok(result) => {
                    send_result = Some(result);
                }
                Err(e) => {
                    #[cfg(feature = "log")]
                    log::error!("send error: {e:?}");
                    once_tx = true;
                }
            }

            #[cfg(feature = "log")]
            log::trace!("{:?}", &result);
        }

        if let Some(tx_result) = send_result {
            if once_rx && sockets.get::<udp::Socket>(udp_handle).can_recv() {
                once_rx = false;

                #[cfg(feature = "log")]
                {
                    let context = NtpContext::new(StdTimestampGen::default());
                    let sock_wrapper = SmoltcpUdpSocketWrapper {
                        socket: RefCell::new(sockets.get_mut::<udp::Socket>(udp_handle)),
                    };
                    let result = sntp_process_response(server_sock_addr, &sock_wrapper, context, tx_result);

                    #[cfg(feature = "log")]
                    log::info!("{result:?}");
                }
            }
        }

        wait(tuntap.as_raw_fd(), iface.poll_delay(Instant::from_secs(1), &sockets)).unwrap();
    }
}

#[cfg(not(unix))]
fn main() {
    panic!("This example supports only Linux platform");
}
