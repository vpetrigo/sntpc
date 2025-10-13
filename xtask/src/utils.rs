//! Utility functions and helpers for build automation
//!
//! This module provides common utilities used across different commands:
//!
//! - [`cargo`] - Cargo command execution helpers and wrappers
//! - [`examples`] - Example project management and categorization
//! - [`output`] - Formatted output and user-friendly display functions
//!
//! These utilities handle cross-cutting concerns like command execution,
//! project discovery, and consistent output formatting across all commands.

/// Cargo command execution utilities
pub mod cargo;
/// Example project management utilities
pub mod examples;
/// Output formatting and display utilities
pub mod output;

// Re-export commonly used utilities
pub use cargo::*;
pub use examples::*;
pub use output::*;
