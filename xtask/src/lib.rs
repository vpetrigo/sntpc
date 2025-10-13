//! # xtask - Build Automation for SNTPC
//!
//! A build automation library for the SNTPC (Simple Network Time Protocol Client) crate and its examples.
//! This crate provides a unified interface for building, testing, checking, and formatting code across
//! the entire SNTPC project following the [xtask pattern](https://github.com/matklad/cargo-xtask).
//!
//! ## Features
//!
//! - **Cross-platform building**: Support for different target platforms and example categories
//! - **Quality assurance**: Integrated testing, linting, and formatting commands
//! - **Dependency isolation**: Each example maintains its own dependencies without conflicts
//! - **Colored output**: User-friendly terminal output with color coding
//! - **Error handling**: Comprehensive error reporting with context
//!
//! ## Usage as a Library
//!
//! While primarily designed as a binary tool, this crate can also be used as a library:
//!
//! ```rust
//! use xtask::{commands, Result};
//!
//! fn main() -> Result<()> {
//!     // Build all examples
//!     commands::build::build_all_examples()?;
//!
//!     // Run tests
//!     commands::test::run_tests()?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Module Structure
//!
//! - [`commands`] - All build automation commands (build, test, check, etc.)
//! - [`utils`] - Common utilities for cargo operations, example management, and output formatting

/// Build automation commands for SNTPC project
///
/// This module contains all the command implementations for building, testing,
/// checking, and maintaining the SNTPC crate and its examples.
pub mod commands;

/// Utility functions and helpers
///
/// This module provides common utilities used across different commands,
/// including cargo command execution, example project management, and
/// formatted output helpers.
pub mod utils;

// Re-export commonly used types and functions
pub use anyhow::{Context, Result};
pub use colored::Colorize;
