[![Crates.io](https://img.shields.io/crates/v/sntpc-net-std)](https://crates.io/crates/sntpc-net-std)
[![docs.rs](https://img.shields.io/badge/docs.rs-sntpc--net--std-66c2a5?logo=docs.rs)](https://docs.rs/sntpc-net-std)
[![License](https://img.shields.io/crates/l/sntpc-net-std)](https://github.com/vpetrigo/sntpc)

# sntpc-net-std

Standard library UDP socket adapter for the [`sntpc`](https://crates.io/crates/sntpc) SNTP client library.

## Design Goal

This crate provides a thin wrapper around `std::net::UdpSocket` that implements the `NtpUdpSocket` trait from `sntpc`.
This separation allows:

- **Independent versioning**: Update `sntpc-net-std` without touching core `sntpc`
- **Minimal dependencies**: Only depends on `std` and `sntpc` core
- **Flexibility**: Users can choose their network stack independently

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
sntpc = "0.8"
sntpc-net-std = "1"
```

## Example

```rust
use sntpc::{sync::get_time, NtpContext, StdTimestampGen};
use sntpc_net_std::UdpSocketWrapper;
use std::net::UdpSocket;

let socket = UdpSocket::bind("0.0.0.0:0").expect("Unable to create UDP socket");
let socket = UdpSocketWrapper::new(socket);
let context = NtpContext::new(StdTimestampGen::default ());

// Use with sntpc functions
```

For complete examples, see the [sntpc examples](https://github.com/vpetrigo/sntpc/tree/master/examples).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

