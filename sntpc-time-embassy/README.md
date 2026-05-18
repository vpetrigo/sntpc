[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/vpetrigo/sntpc/ci.yml?logo=github)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/sntpc-time-embassy)](https://crates.io/crates/sntpc-time-embassy)
[![docs.rs](https://img.shields.io/badge/docs.rs-sntpc--time--embassy-66c2a5?logo=docs.rs)](https://docs.rs/sntpc-time-embassy)

# sntpc-time-embassy

[`NtpTimestampGenerator`](https://docs.rs/sntpc/latest/sntpc/trait.NtpTimestampGenerator.html) implementation backed by [`embassy-time`](https://crates.io/crates/embassy-time) for the [`sntpc`](https://crates.io/crates/sntpc) SNTP client library.

## Design Goal

This crate provides an [`EmbassyTimestampGenerator`] that implements the `NtpTimestampGenerator` trait from `sntpc` using the Embassy async runtime's monotonic clock. This separation allows:

- **Independent versioning**: Update `embassy-time` without requiring `sntpc` core updates
- **Version flexibility**: Works with `embassy-time` 0.5.x (`>=0.5, <0.6`)
- **Embedded focus**: Minimal dependencies suitable for `no_std` embedded systems
- **Clean separation**: Core SNTP protocol logic remains independent of the async runtime

## Important

`EmbassyTimestampGenerator` provides **monotonic** timestamps based on `embassy_time::Instant`, not wall-clock time. This is sufficient for SNTP request/response delay calculations, where only the relative elapsed time between request and response matters. The actual wall-clock offset is computed by the `sntpc` library using the server's timestamps.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
sntpc = { version = "0.10", default-features = false }
sntpc-time-embassy = "0.5"
embassy-time = ">=0.5,<0.6"
```

## Example

```rust
use sntpc::{get_time, NtpContext};
use sntpc_time_embassy::EmbassyTimestampGenerator;

// Create an NtpContext with the Embassy timestamp generator
let ntp_context = NtpContext::new(EmbassyTimestampGenerator::default());

// Use with an Embassy UDP socket adapter
let result = get_time(server_addr, &socket, ntp_context).await;
```

For complete examples, see the [sntpc examples](https://github.com/vpetrigo/sntpc/tree/master/examples/embassy-net).

## Compatibility

- **sntpc**: 0.10.x
- **embassy-time**: 0.5.x (any version >= 0.5, < 0.6)
- **no_std**: Fully supported

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or
<a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this codebase by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>