use criterion::{criterion_group, criterion_main, Criterion};
use sntpc::get_time;
use sntpc::{NtpContext, StdTimestampGen};
use std::hint::black_box;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

fn criterion_benchmark(c: &mut Criterion) {
    let socket =
        UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))).unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 1231));
    let context = NtpContext::new(StdTimestampGen::default());
    let mut executor = miniloop::executor::Executor::new();

    c.bench_function("async_sntp_client", |b| {
        b.iter(|| {
            black_box(executor.block_on(get_time(addr, &socket, context)))
        });
    });
}

criterion_group!(sync_benches, criterion_benchmark);
criterion_main!(sync_benches);
