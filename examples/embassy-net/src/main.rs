//! Demonstrates how to use [`embassy-net`] with the [`sntpc`] library.
//!
//! This example fetches the current time from a NTP server using the
//! SNTP client library and prints the result.
//!
//! ## Create a TUN/TAP interface
//!
//! ```sh
//! sudo ip tuntap add name tap0 mode tap
//! sudo ip link set tap0 up
//! sudo ip addr add 192.168.69.1/24 dev tap0
//!
//! # Enable IP forwarding
//! sudo sysctl -w net.ipv4.ip_forward=1
//!
//! # Enable NAT for the tap0 interface
//! export DEFAULT_IFACE=$(ip route show default | grep -oP 'dev \K\S+')
//! sudo iptables -A FORWARD -i tap0 -j ACCEPT
//! sudo iptables -A FORWARD -o ${DEFAULT_IFACE} -j ACCEPT
//! sudo iptables -t nat -A POSTROUTING -o ${DEFAULT_IFACE} -j MASQUERADE
//! ```
//!
//! ## Run the example
//!
//! ```sh
//! cargo build --features "log"
//! sudo ../../target/debug/example-embassy-net
//! ```
//!
//! ## Cleanup
//!
//! To remove the TUN/TAP interface, run:
//!
//! ```sh
//! sudo ip link del tap0
//! ```

use embassy_executor::{Executor, Spawner};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config, IpEndpoint, Ipv4Address, Ipv4Cidr, StackResources};
use embassy_net_tuntap::TunTapDevice;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use sntpc::{async_impl::get_time, NtpContext, NtpTimestampGenerator};
use static_cell::StaticCell;

#[cfg(feature = "log")]
use log::{error, info};

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn main_task(spawner: Spawner) {
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    // Create TUN/TAP device
    let device = TunTapDevice::new("tap0").unwrap();

    // Configure network stack
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
        dns_servers: Vec::from_slice(&[Ipv4Address::new(8, 8, 8, 8)]).unwrap(),
        gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    });

    // Init network stack
    let (stack, runner) = embassy_net::new(
        device,
        config,
        RESOURCES.init(StackResources::new()),
        0,
    );

    // Launch network task
    spawner.spawn(net_task(runner)).unwrap();

    // Wait for the tap interface to be up before continuing
    stack.wait_config_up().await;

    // Create UDP socket
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(123).unwrap();

    let context = NtpContext::new(StdTimestampGen::default());

    let ntp_addrs = stack
        .dns_query("pool.ntp.org", DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");
    if ntp_addrs.is_empty() {
        #[cfg(feature = "log")]
        error!("Failed to resolve DNS");
        return;
    }

    loop {
        let result =
            get_time(IpEndpoint::new(ntp_addrs[0], 123), &socket, context)
                .await
                .unwrap();

        #[cfg(feature = "log")]
        info!("Time: {:?}", result);

        Timer::after(Duration::from_secs(15)).await;
    }
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

fn main() {
    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(main_task(spawner)).unwrap();
    });
}

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
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
