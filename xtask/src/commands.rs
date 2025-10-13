//! Build automation commands for the SNTPC project
//!
//! This module contains all command implementations organized by functionality:
//!
//! - [`build`] - Building the main crate and various example categories
//! - [`test`] - Running tests for the main crate
//! - [`check`] - Code checking across all projects
//! - [`clippy`] - Linting with clippy and strict rules
//! - [`format`] - Code formatting operations
//! - [`clean`] - Cleanup of build artifacts
//!
//! Each command module provides specific functionality while sharing common
//! utilities from the [`crate::utils`] module.

/// Building commands for main crate and examples
pub mod build;
/// Code checking functionality
pub mod check;
/// Build artifact cleanup
pub mod clean;
/// Clippy linting commands
pub mod clippy;
/// Code formatting operations
pub mod format;
/// Test execution commands
pub mod test;

// Re-export all command functions for easier access
pub use build::*;
pub use check::*;
pub use clean::*;
pub use clippy::*;
pub use format::*;
pub use test::*;
