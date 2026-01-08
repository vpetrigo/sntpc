[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/vpetrigo/sntpc/ci.yml?logo=github)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sntpc-net-embassy)](https://crates.io/crates/sntpc-net-embassy)
[![docs.rs](https://img.shields.io/badge/docs.rs-sntpc--net--embassy-66c2a5?logo=docs.rs)](https://docs.rs/sntpc-net-embassy)

# sntpc-net-embassy

Embassy async runtime UDP socket adapter for the [`sntpc`](https://crates.io/crates/sntpc) SNTP client library.

## Design Goal

This crate provides a wrapper around `embassy_net::udp::UdpSocket` that implements the `NtpUdpSocket` trait from `sntpc`. This separation allows:

- **Independent versioning**: Update embassy-net without requiring `sntpc` core updates
- **Version flexibility**: Works with embassy-net 0.7.x (`>=0.7, <0.8`)
- **Embedded focus**: Minimal dependencies suitable for `no_std` embedded systems
- **Future compatibility**: When embassy-net 0.8+ releases, only this adapter needs updating

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
sntpc = { version = "0.8", default-features = false }
sntpc-net-embassy = { version = "0.7", default-features = false }
embassy-net = { version = "0.7", features = ["udp", "proto-ipv4"] }
```

## Features

- `ipv6`: Enable IPv6 protocol support (propagates to `embassy-net`)
- `log`: Enable logging support via the `log` crate
- `defmt`: Enable logging support via the `defmt` crate (for embedded systems)

**Note**: `log` and `defmt` are mutually exclusive. If both are enabled, `defmt` takes priority.

## Example

```rust
use sntpc::{get_time, NtpContext};
use sntpc_net_embassy::UdpSocketWrapper;
use embassy_net::udp::UdpSocket;

// Within an embassy async context
let socket = UdpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
// binding and other required steps
let socket = UdpSocketWrapper::from(socket);

let result = get_time(server_addr, &socket, ntp_context).await;
```

For complete examples, see the [sntpc examples](https://github.com/vpetrigo/sntpc/tree/master/examples/embassy-net).

## Compatibility

- **sntpc**: 0.8.x
- **embassy-net**: 0.7.x (any version >= 0.7, < 0.8)
- **no_std**: Fully supported

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

