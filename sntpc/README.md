[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/vpetrigo/sntpc/ci.yml?logo=github)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![](https://img.shields.io/crates/v/sntpc)](https://crates.io/crates/sntpc)
[![docs.rs](https://img.shields.io/badge/docs.rs-sntpc-66c2a5?logo=docs.rs&label=docs.rs)](https://docs.rs/sntpc)
[![codecov](https://codecov.io/gh/vpetrigo/sntpc/graph/badge.svg?token=dZ6iBIsSih)](https://codecov.io/gh/vpetrigo/sntpc)

# Simple Rust SNTP client

-------------------------

This crate provides a method for sending requests to NTP servers and process responses,
extracting received timestamp.

Supported SNTP protocol versions:

- [SNTPv4](https://datatracker.ietf.org/doc/html/rfc4330)

### Usage example

- dependency for the app

```toml
[dependencies]
sntpc = { version = "0.7", features = ["sync"] }
```

- application code

```rust
use sntpc::{sync::get_time, NtpContext, StdTimestampGen};

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org:123";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com:123";

fn main() {
    #[cfg(feature = "log")]
    if cfg!(debug_assertions) {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    } else {
        simple_logger::init_with_level(log::Level::Info).unwrap();
    }

    let socket =
        UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("Unable to set UDP socket read timeout");

    for addr in POOL_NTP_ADDR.to_socket_addrs().unwrap() {
        let ntp_context = NtpContext::new(StdTimestampGen::default());
        let result = get_time(addr, &socket, ntp_context);

        match result {
            Ok(time) => {
                assert_ne!(time.sec(), 0);
                let seconds = time.sec();
                let microseconds = u64::from(time.sec_fraction()) * 1_000_000
                    / u64::from(u32::MAX);
                println!("Got time from [{POOL_NTP_ADDR}] {addr}: {seconds}.{microseconds}");

                break;
            }
            Err(err) => println!("Err: {err:?}"),
        }

        thread::sleep(Duration::new(2, 0));
    }
}
```

You can find this [example](examples/simple-request) as well as other example projects in the
[example directory](examples).

## `no_std` support

There is an example available on how to use [`smoltcp`](examples/smoltcp-request) stack and that should provide
general idea on how to bootstrap `no_std` networking and timestamping tools for `sntpc` library usage

## `async` support

Starting version `0.5` the default interface is `async`. If you want to use synchronous interface, read about `sync`
feature below.

`tokio` example: [`examples/tokio`](examples/tokio)

There is also `no_std` support with feature `async`, but it requires Rust >= `1.75-nightly` version.
The example can be found in [separate repository](https://github.com/vpikulik/sntpc_embassy).

## `sync` support

`sntpc` crate is `async` by default, since most of the frameworks (I have seen) for embedded systems utilize
asynchronous approach, e.g.:

- [RTIC](https://github.com/rtic-rs/rtic)
- [embassy](https://github.com/embassy-rs/embassy)

If you need fully synchronous interface it is available in the `sntpc::sync` submodule and respective `sync`-feature
enabled. In the case someone needs a synchronous socket support the currently async `NtpUdpSocket` trait can be
implemented in a fully synchronous manner. This is an example for the `std::net::UdpSocket` that is available in the
crate:

```rust
#[cfg(feature = "std")]
impl NtpUdpSocket for UdpSocket {
    async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
        match self.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }

    async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
```

As you can see, you may implement everything as synchronous, `sntpc` synchronous interface handles `async`-like stuff
internally.

That approach also allows to avoid issues with [`maybe_async`](https://docs.rs/maybe-async/latest/maybe_async/) when the
sync/async feature [violates Cargo requirements](https://doc.rust-lang.org/cargo/reference/features.html):
> That is, enabling a feature should not disable functionality, and it should usually be safe to enable any combination
> of features.

Small overhead introduced by creating an executor should be negligible.

# Contribution

Contributions are always welcome! If you have an idea, it's best to float it by me before working on it to ensure no
effort is wasted. If there's already an open issue for it, knock yourself out. See the
[**contributing section**](CONTRIBUTING.md) for additional details

## Thanks

1. [Frank A. Stevenson](https://github.com/snakehand): for implementing stricter adherence to RFC4330 verification
   scheme
2. [Timothy Mertz](https://github.com/mertzt89): for fixing possible overflow in offset calculation
3. [HannesH](https://github.com/HannesGitH): for fixing a typo in the README.md
4. [Richard Penney](https://github.com/rwpenney): for adding two indicators of the NTP server's accuracy into the
   `NtpResult` structure
5. [Vitali Pikulik](https://github.com/vpikulik): for adding `async` support
6. [tsingwong](https://github.com/tsingwong): for fixing invalid link in the `README.md`
7. [Robert Bastian](https://github.com/robertbastian): for fixing the overflow issue in the `calculate_offset`
8. [oleid](https://github.com/oleid): for bringing `embassy` socket support
9. [Damian Peckett](https://github.com/dpeckett): for adding `defmt` support and elaborating on `embassy` example
10. [icalder](https://github.com/icalder): for improving `embassy-net` support and adding missing `defmt` format support
    for some `sntpc` types

Really appreciate all your efforts! Please [let me know](mailto:vladimir.petrigo@gmail.com) if I forgot someone.

# License

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
