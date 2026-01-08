[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/vpetrigo/sntpc/ci.yml?logo=github)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sntpc-net-tokio)](https://crates.io/crates/sntpc-net-tokio)
[![docs.rs](https://img.shields.io/badge/docs.rs-sntpc--net--tokio-66c2a5?logo=docs.rs)](https://docs.rs/sntpc-net-tokio)

# sntpc-net-tokio

Tokio async runtime UDP socket adapter for the [`sntpc`](https://crates.io/crates/sntpc) SNTP client library.

## Design Goal

This crate provides a wrapper around `tokio::net::UdpSocket` that implements the `NtpUdpSocket` trait from `sntpc`. This separation allows:

- **Independent versioning**: Update tokio without requiring `sntpc` core updates
- **Version flexibility**: Works with any tokio 1.x version (`>=1, <2`)
- **Clean separation**: Core SNTP protocol logic remains independent of async runtime
- **Future compatibility**: When tokio 2.x releases, only this adapter needs updating

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
sntpc = "0.8"
sntpc-net-tokio = "1"
tokio = { version = "1", features = ["net", "rt"] }
```

## Example

```rust
use sntpc::{get_time, NtpContext, StdTimestampGen};
use sntpc_net_tokio::UdpSocketWrapper;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Socket creation");
    let socket = UdpSocketWrapper::from(socket);
    let context = NtpContext::new(StdTimestampGen::default());
    
    let result = get_time("pool.ntp.org:123".parse().unwrap(), &socket, context).await;
    println!("Time: {:?}", result);
}
```

For complete examples, see the [sntpc examples](https://github.com/vpetrigo/sntpc/tree/master/examples/tokio).

## Compatibility

- **sntpc**: 0.8.x
- **tokio**: 1.x (any version >= 1.0, < 2.0)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

