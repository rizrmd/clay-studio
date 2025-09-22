// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

// Warn on unwrap and expect usage to encourage proper error handling
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
// Allow dead code during development/refactoring
#![allow(dead_code)]

pub mod api;
pub mod core;
pub mod models;
pub mod utils;

// Re-export commonly used types for convenience
pub use core::claude::{
    ClaudeManager, ClaudeMessage, ClaudeSDK, ClaudeSetup, QueryOptions, QueryRequest,
};

pub use utils::AppError;
