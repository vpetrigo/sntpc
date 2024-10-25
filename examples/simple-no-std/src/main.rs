#![no_std]
#![no_main]
use core::future::Future;
use core::task::Poll;
use sntpc::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use sntpc::{
    async_impl::{get_time, NtpUdpSocket},
    NtpContext, NtpTimestampGenerator, Result,
};

#[derive(Copy, Clone, Default)]
struct TimestampGen {
    duration: u64,
}

impl NtpTimestampGenerator for TimestampGen {
    fn init(&mut self) {
        self.duration = 0u64;
    }

    fn timestamp_sec(&self) -> u64 {
        self.duration >> 32
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        (self.duration & 0xffffffffu64) as u32
    }
}

#[derive(Debug)]
struct SimpleUdp;

impl NtpUdpSocket for SimpleUdp {
    fn send_to<T: ToSocketAddrs + Send>(
        &self,
        _buf: &[u8],
        _addr: T,
    ) -> impl Future<Output = Result<usize>> {
        core::future::ready(Ok(0))
    }

    fn recv_from(
        &self,
        _buf: &mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> {
        core::future::ready(Ok((
            0usize,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 123u16),
        )))
    }
}

async fn body() -> Poll<Result<i32>> {
    let timestamp_gen = TimestampGen::default();
    let context = NtpContext::new(timestamp_gen);
    let socket = SimpleUdp;
    let address = (IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 123u16);

    match get_time(address, socket, context).await {
        Ok(time) => {
            assert_ne!(time.sec(), 0);
            let _seconds = time.sec();
            let _microseconds =
                time.sec_fraction() as u64 * 1_000_000 / u32::MAX as u64;

            Poll::Ready(Ok(0))
        }
        Err(err) => Poll::Ready(Err(err)),
    }
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn Reset() -> ! {
    main()
}

#[no_mangle]
pub extern "C" fn start() {
    main()
}

#[no_mangle]
pub extern "C" fn WinMain() {
    main()
}

fn main() -> ! {
    loop {
        let _ = async move {
            let _ = body().await;
        };
    }
}
