pub mod build;
pub mod check;
pub mod clean;
pub mod clippy;
pub mod format;
pub mod test;

// Re-export all command functions for easier access
pub use build::*;
pub use check::*;
pub use clean::*;
pub use clippy::*;
pub use format::*;
pub use test::*;
