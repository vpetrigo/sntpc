#![no_std]
#![no_main]
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::future::Future;
use core::net::{IpAddr, Ipv4Addr};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use miniloop::{executor::Executor, task::Task};
use sntpc::net::SocketAddr;
use sntpc::{
    get_time, NtpContext, NtpTimestampGenerator, NtpUdpSocket, Result,
};

const ARENA_SIZE: usize = 128 * 1024;
const MAX_SUPPORTED_ALIGN: usize = 4096;
#[repr(C, align(4096))] // 4096 == MAX_SUPPORTED_ALIGN
struct SimpleAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remaining: AtomicUsize, // we allocate from the top, counting down
}

unsafe impl Sync for SimpleAllocator {}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // `Layout` contract forbids making a `Layout` with align=0, or align not power of 2.
        // So we can safely use a mask to ensure alignment without worrying about UB.
        let align_mask_to_round_down = !(align - 1);

        if align > MAX_SUPPORTED_ALIGN {
            return null_mut();
        }

        let mut allocated = 0;
        if self
            .remaining
            .fetch_update(Relaxed, Relaxed, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return null_mut();
        };
        self.arena.get().cast::<u8>().add(allocated)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        todo!()
    }

    unsafe fn realloc(
        &self,
        _ptr: *mut u8,
        _layout: Layout,
        _new_size: usize,
    ) -> *mut u8 {
        todo!()
    }
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remaining: AtomicUsize::new(ARENA_SIZE),
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
        (self.duration & 0xff_ff_ff_ffu64) as u32
    }
}

#[derive(Debug)]
struct SimpleUdp;

impl NtpUdpSocket for SimpleUdp {
    fn send_to(
        &self,
        _buf: &[u8],
        _addr: SocketAddr,
    ) -> impl Future<Output = Result<usize>> {
        core::future::ready(Ok(48))
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

async fn body() -> Result<i32> {
    let timestamp_gen = TimestampGen::default();
    let context = NtpContext::new(timestamp_gen);
    let socket = SimpleUdp;
    let address = (IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 123u16).into();

    match get_time(address, &socket, context).await {
        Ok(time) => {
            assert_ne!(time.sec(), 0);
            let _seconds = time.sec();
            let _microseconds = u64::from(time.sec_fraction()) * 1_000_000
                / u64::from(u32::MAX);

            Ok(0)
        }
        Err(err) => Err(err),
    }
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

/// # Safety
///
/// This is a definition of an entry point of a program that should not be called directly
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    main()
}

#[no_mangle]
pub extern "C" fn WinMain() {
    main()
}

fn main() -> ! {
    let mut executor = Executor::new();
    let mut task = Task::new("sntp", body());
    let mut handler = task.create_handle();

    let result = executor.spawn(&mut task, &mut handler);

    assert!(result.is_ok(), "Failed to spawn task");
    executor.run();
    assert!(handler.value.is_some(), "Task has not completed");
    panic!("Done");
}
