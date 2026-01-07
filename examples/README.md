# SNTPC Examples

This directory contains examples demonstrating various use cases of the `sntpc` library.

## Basic Usage

- [`simple-request`](simple-request/) - Basic synchronous SNTP request using std UDP socket with `sync` feature
- [`tokio`](tokio/) - Async SNTP client using tokio runtime and `sntpc-net-tokio` adapter

## Embedded / no_std

- [`simple-no-std`](simple-no-std/) - Minimal no_std example with custom allocator and mock implementations
- [`embassy-net`](embassy-net/) - Using embassy async runtime with TAP interface (requires Linux setup)
- [`embassy-net-timeout`](embassy-net-timeout/) - Embassy example with timeout handling using `embassy-time`
- [`smoltcp-request`](smoltcp-request/) - Custom TCP/IP stack using smoltcp with TAP interface (requires Linux setup)

## Advanced Features

- [`timesync`](timesync/) - System time synchronization using the `utils` feature (requires elevated privileges)

### Platform-Specific Requirements

Some examples require additional system setup:

- `embassy-net`, `embassy-net-timeout`, `smoltcp-request`: Require TAP interface setup on Linux (see individual
  example documentation)
- `timesync`: Requires elevated privileges (sudo) to modify system time
- `simple-no-std`: Demonstrates no_std usage but still requires a host OS to run

See individual example directories for detailed setup instructions and requirements.
