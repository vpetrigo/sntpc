[![sntpc test](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml/badge.svg)](https://github.com/vpetrigo/sntpc/actions/workflows/ci.yml)
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

More information about this crate can be found in the [crate documentation](https://docs.rs/sntpc)

### Usage example

```rust
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
const POOL_NTP_ADDR: &str = "pool.ntp.org:123";
#[allow(dead_code)]
const GOOGLE_NTP_ADDR: &str = "time.google.com:123";

fn main() {
    for _ in 0..5 {
        let socket =
            UdpSocket::bind("0.0.0.0:0").expect("Unable to crate UDP socket");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("Unable to set UDP socket read timeout");

        let result = sntpc::simple_get_time(POOL_NTP_ADDR, &socket);

        match result {
            Ok(time) => {
                assert_ne!(time.sec(), 0);
                let seconds = time.sec();
                let microseconds =
                    u64::from(time.sec_fraction()) * 1_000_000 / u64::from(u32::MAX);
                println!("Got time: {seconds}.{microseconds}");
            }
            Err(err) => println!("Err: {err:?}"),
        }

        thread::sleep(Duration::new(15, 0));
    }
}
```

You can find this [example](examples/simple-request) as well as other example projects in the
[example directory](examples).

## `no_std` support

-------------------

Currently, there are basic `no_std` support available, thanks to [`no-std-net`](https://crates.io/crates/no-std-net)
crate. There is an example available on how to use [`smoltcp`][smoltcp] stack and that should provide
general idea on how to bootstrap `no_std` networking and timestamping tools for `sntpc` library usage

## `async` support

-------------------

Feature `async_tokio` allows to use crate together with [tokio](https://docs.rs/tokio/latest/tokio/).
There is an example: [`examples/tokio.rs`](examples/tokio.rs).

There is also `no_std` support with feature `async`, but it requires Rust >= `1.75-nightly` version.
The example can be found in [separate repository](https://github.com/vpikulik/sntpc_embassy).

# Contribution

--------------

Contributions are always welcome! If you have an idea, it's best to float it by me before working on it to ensure no
effort is wasted. If there's already an open issue for it, knock yourself out. See the
[**contributing section**](CONTRIBUTING.md) for additional details

## Thanks

1. [Frank A. Stevenson](https://github.com/snakehand): for implementing stricter adherence to RFC4330 verification scheme
2. [Timothy Mertz](https://github.com/mertzt89): for fixing possible overflow in offset calculation
3. [HannesH](https://github.com/HannesGitH): for fixing a typo in the README.md
4. [Richard Penney](https://github.com/rwpenney): for adding two indicators of the NTP server's accuracy into the `NtpResult` structure 
5. [Vitali Pikulik](https://github.com/vpikulik): for adding `async` support
6. [tsingwong](https://github.com/tsingwong): for fixing invalid link in the `README.md`
7. [Robert Bastian](https://github.com/robertbastian): for fixing the overflow issue in the `calculate_offset`

Really appreciate all your efforts! Please [let me know](mailto:vladimir.petrigo@gmail.com) if I forgot someone.

# License

---------

<sup>
This project is licensed under <a href="LICENSE.md">The 3-Clause BSD License</a>
</sup>

<br/>

<sup>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in time by you, as
defined in the 3-Clause BSD license, shall be licensed as above, without any additional terms or
conditions.
</sup>
