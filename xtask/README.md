This directory contains the build automation tool (`xtask`) for the sntpc project. It provides commands to build different categories of examples independently, avoiding dependency conflicts that can occur in workspace setups.
cargo xtask build-unix            # Build Unix-specific examples (all except simple-no-std)
cargo xtask build-cross-platform  # Build cross-platform examples (simple-request, tokio, timesync)

# Build everything
cargo xtask build-all             # Build all examples
cargo xtask build-crate           # Build only the main sntpc crate

# Testing and checking
cargo xtask test                  # Run tests for main crate
cargo xtask check                 # Check all code (main crate + examples)

# Cleanup
cargo xtask clean                 # Clean all build artifacts
```

## Example Categories

### No-std Examples
- `simple-no-std` - Basic SNTP client for embedded environments

These examples are built with the `thumbv7em-none-eabihf` target by default.

### Unix-specific Examples  
- `simple-request` - Basic synchronous SNTP request
- `tokio` - Async SNTP client using tokio runtime
- `embassy-net` - Embassy networking example
- `embassy-net-timeout` - Embassy with timeout handling
- `smoltcp-request` - Using smoltcp network stack
- `timesync` - Time synchronization example

### Cross-platform Examples
- `simple-request` - Works on any platform with std
- `tokio` - Cross-platform async example
- `timesync` - Platform-agnostic time sync

## Benefits of xtask Approach

1. **Dependency Isolation**: Each example maintains its own dependencies without conflicts
2. **Environment Detection**: Automatically handles platform-specific requirements
3. **Selective Building**: Build only what's compatible with your environment
4. **CI Integration**: Clean separation of build jobs for different environments
5. **Developer Experience**: Simple, memorable commands
xtask = "run --package xtask --"

[build]
# Default target for no-std examples
# Can be overridden with --target flag
rustflags = ["-C", "link-arg=-Tlink.x"]

[target.thumbv7em-none-eabihf]
# ARM Cortex-M4 target for no-std examples
runner = "echo 'Build completed for thumbv7em-none-eabihf target'"
