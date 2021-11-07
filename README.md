[![sntpc test](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml/badge.svg)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
[![Build Status](https://travis-ci.com/vpetrigo/sntpc.svg?branch=master)](https://travis-ci.com/vpetrigo/sntpc)
[![](https://img.shields.io/crates/v/sntpc)](https://crates.io/crates/sntpc)
[![](https://img.shields.io/crates/l/sntpc)](https://github.com/vpetrigo/sntpc/blob/master/LICENSE.md)

# Simple Rust NTP client

This crate provides a method for sending requests to NTP servers
and process responses, extracting received timestamp

### Documentation

https://docs.rs/sntpc

### Installation

This crate works with Cargo and is on
[crates.io](https://crates.io/crates/sntpc). Add it to your `Cargo.toml`
like so:

```toml
[dependencies]
sntpc = "0.2"
```

By calling the `request()` method and providing a proper NTP pool or server you
should get a valid synchronization timestamp:

```rust
use sntpc;

let result = sntpc::request("pool.ntp.org", 123);
if let Ok(sntpc::NtpResult {
    sec, nsec, roundtrip, offset
}) = result {
    println!("NTP server time: {}.{}", sec, nsec);
    println!("Roundtrip time: {}, offset: {}", roundtrip, offset);
}
```

## Lightweight system time synchronization

The `sntpc` crate contains the `timesync` application that may sync system
time with the given NTP server

### Command-line options

```
USAGE:
    timesync [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --port <port>        NTP server port [default: 123]
    -s, --server <server>    NTP server hostname [default: time.google.com]
```

This is the output of `timesync -h`.
