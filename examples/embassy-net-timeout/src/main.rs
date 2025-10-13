//! Demonstrates how to use [`embassy-net`] with the [`sntpc`] library and handle potential
//! socket timeout events.
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
//! cargo build
//! sudo ../../target/debug/example-embassy-net
//! ```
//!
//! To view logs, run:
//!
//! ```sh
//! defmt-print -e ../../target/debug/example-embassy-net tcp
//! ```
//! You will need the defmt-print tool installed. You can install it by running:
//!
//! ```sh
//! cargo install defmt-print
//! ```
//!
//! ## Cleanup
//!
//! To remove the TUN/TAP interface, run:
//!
//! ```sh
//! sudo ip link del tap0
//! ```
macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(unix)]
            $item
        )*
    };
}

macro_rules! cfg_win {
    ($($item:item)*) => {
        $(
            #[cfg(windows)]
            $item
        )*
    };
}

cfg_unix! {
    use embassy_executor::task;
    use embassy_time::{Duration, with_timeout};

    use embassy_net::udp::{PacketMetadata, UdpSocket};
    use embassy_net::{Config, Ipv4Address, Ipv4Cidr, StackResources};
    use embassy_net_tuntap::TunTapDevice;
    use heapless::Vec;
    use rand_core::{OsRng, TryRngCore};
    use static_cell::StaticCell;
    use std::mem::transmute;

    use sntpc::{NtpContext, StdTimestampGen, get_time};

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    const SNTP_POOL: &str = "192.168.1.111:1234";

    #[task]
    async fn net_task(mut runner: embassy_net::Runner<'static, TunTapDevice>) -> ! {
        runner.run().await
    }

    #[task]
    async fn periodic_time_fetch(spawner: embassy_executor::Spawner) {
        // Init network device
        let device = TunTapDevice::new("tap0").unwrap();
        // Choose between dhcp or static ip
        let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
            dns_servers: Vec::new(),
            gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
        });

        // Generate random seed
        let mut seed = [0; 8];
        OsRng.try_fill_bytes(&mut seed).unwrap();
        let seed = u64::from_le_bytes(seed);

        // Init network stack
        let (stack, runner) = embassy_net::new(
            device,
            config,
            RESOURCES.init(StackResources::new()),
            seed,
        );

        // Launch network task
        spawner
            .spawn(net_task(runner))
            .expect("Failed to spawn network task");

        // Then we can use it!
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
        socket.bind(9400).unwrap();

        loop {
            let ntp_context = NtpContext::new(StdTimestampGen::default());

            let result = with_timeout(Duration::from_secs(1), {
                get_time(
                    SNTP_POOL.parse().expect("Invalid address"),
                    &socket,
                    ntp_context,
                )
            })
            .await;
            println!("Got time: {result:?}");
        }
    }

    fn main() -> ! {
        let mut executor = embassy_executor::Executor::new();
        let static_executor: &'static mut embassy_executor::Executor =
            unsafe { transmute(&mut executor) };

        static_executor.run(|spawner| {
            spawner.spawn(periodic_time_fetch(spawner)).unwrap();
        });
    }
}

cfg_win! {
    fn main() {
        panic!("This example is not supported on Windows");
    }
}
