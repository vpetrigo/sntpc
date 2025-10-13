# Build Automation for `sntpc` crate

A build automation tool for the `sntpc` (Simple Network Time Protocol Client) crate and its examples.

## Overview

`xtask` is a Rust-based task runner that provides a unified interface for building, testing, checking, and formatting
code across the entire SNTPC project. It follows the [xtask pattern](https://github.com/matklad/cargo-xtask) for project
automation.

## Installation

From the project root directory:

```bash
cargo build --package xtask --release
```

Or run directly with cargo:

```bash
cargo run --package xtask -- <command>
```

## Usage

```bash
cargo xtask <COMMAND>
```

### Available Commands

#### Build Commands

- **`build-nostd`** - Build no-std examples (simple-no-std)
- **`build-unix`** - Build Unix-specific examples (all except simple-no-std)
- **`build-cross-platform`** - Build cross-platform examples (simple-request, tokio, timesync)
- **`build-all`** - Build all examples
- **`build-crate`** - Build the main sntpc crate
    - `--all-features` - Build with all features enabled
    - `--no-default-features` - Build with no default features

#### Quality Assurance Commands

- **`test`** - Run tests for the main crate
- **`check`** - Check all code (main crate and examples)
- **`clippy`** - Run clippy on all code with strict linting
- **`format`** - Check code formatting for the main crate and all examples
    - `--check` - Check formatting without making changes
    - `--fix` - Fix formatting issues

#### Maintenance Commands

- **`clean`** - Clean all build artifacts

### Examples

```bash
# Build all examples
cargo xtask build-all

# Run tests
cargo xtask test

# Check formatting
cargo xtask format --check

# Fix formatting issues
cargo xtask format --fix

# Run clippy with strict linting
cargo xtask clippy

# Build main crate with all features
cargo xtask build-crate --all-features

# Clean all build artifacts
cargo xtask clean
```

## Project Structure

The xtask crate is organized into several modules:

- **`commands/`** - Implementation of all build automation commands
    - `build.rs` - Building examples and main crate
    - `check.rs` - Code checking functionality
    - `clean.rs` - Cleanup operations
    - `clippy.rs` - Linting with clippy
    - `format.rs` - Code formatting operations
    - `test.rs` - Test execution

- **`utils/`** - Common utilities used across commands
    - `cargo.rs` - Cargo command execution helpers
    - `examples.rs` - Example project management
    - `output.rs` - Output formatting and display
