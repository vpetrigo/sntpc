[![sntpc test](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml/badge.svg)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![Build Status](https://travis-ci.com/vpetrigo/sntpc.svg?branch=master)](https://travis-ci.com/vpetrigo/sntpc)
[![](https://img.shields.io/crates/v/sntpc)](https://crates.io/crates/sntpc)
[![](https://img.shields.io/crates/l/sntpc)](https://github.com/vpetrigo/sntpc/blob/master/LICENSE.md)

# Simple Rust SNTP client

-------------------------

This crate provides a method for sending requests to NTP servers and process responses,
extracting received timestamp.

Supported SNTP protocol versions:
- [SNTPv4](https://datatracker.ietf.org/doc/html/rfc4330)

### Documentation

-----------------

https://docs.rs/sntpc

### Installation

----------------

This crate works with Cargo and is on
[crates.io](https://crates.io/crates/sntpc). Add it to your `Cargo.toml`
like so:

```toml
[dependencies]
sntpc = "0.3.2"
```

By calling the `get_time()` method and providing a proper NTP pool or server you
should get a valid synchronization timestamp:

```rust
use sntpc::{Error, NtpContext, NtpTimestampGenerator, NtpUdpSocket, Result};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: Duration,
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

#[derive(Debug)]
struct UdpSocketWrapper(UdpSocket);

impl NtpUdpSocket for UdpSocketWrapper {
    fn send_to<T: ToSocketAddrs>(
        &self,
        buf: &[u8],
        addr: T,
    ) -> Result<usize> {
        match self.0.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(Error::Network),
        }
    }
    fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(Error::Network),
        }
    }
}
fn main() {
    let socket =
        UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
    socket
       .set_read_timeout(Some(Duration::from_secs(2)))
       .expect("Unable to set UDP socket read timeout");
    let sock_wrapper = UdpSocketWrapper(socket);
    let ntp_context = NtpContext::new(StdTimestampGen::default());
    let result =
        sntpc::get_time("time.google.com:123", sock_wrapper, ntp_context);

    match result {
       Ok(time) => {
           println!("Got time: {}.{}", time.sec(), time.nsec());
       }
       Err(err) => println!("Err: {:?}", err),
    }
 }
```

## `no_std` support

-------------------

Currently there are basic `no_std` support available, thanks [`no-std-net`](https://crates.io/crates/no-std-net)
crate. There is an example available on how to use [`smoltcp`][smoltcp] stack and that should provide
general idea on how to bootstrap `no_std` networking and timestamping tools for `sntpc` library usage

# Examples

----------

You can find several examples that shows how to use the library in details under [examples/] folder.
Currently there are examples that show:
- usage of SNTP library in `std` environment
- usage of SNTP library with [`smoltcp`][smoltcp] TCP/IP stack. Some `std` dependencies
required only due to smoltcp available interfaces

[smoltcp]: https://github.com/smoltcp-rs/smoltcp

# Contribution

--------------

Contributions are always welcome! If you have an idea, it's best to float it by me before working on it to ensure no
effort is wasted. If there's already an open issue for it, knock yourself out. See the
[**contributing section**](CONTRIBUTING.md) for additional details

# License

---------

This project is licensed under:

- [The 3-Clause BSD License](LICENSE.md)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in time by you, as
defined in the 3-Clause BSD License license, shall be licensed as above, without any additional terms or
conditions.
