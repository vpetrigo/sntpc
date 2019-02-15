[![Build Status](https://travis-ci.com/vpetrigo/sntpc.svg?branch=master)](https://travis-ci.com/vpetrigo/sntpc)
[![](http://meritbadge.herokuapp.com/sntpc)](https://crates.io/crates/sntpc)

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
sntpc = "0.1"
```

By calling the `request()` method and providing a proper NTP pool or server you
should get a valid synchronization timestamp:

```rust
use sntpc;

let result = sntpc::request(POOL_NTP_ADDR, 123);

if let Ok(timestamp) = result {
    assert_ne!(timestamp, 0);
    println!("Timestamp: {}", timestamp);
}
```
