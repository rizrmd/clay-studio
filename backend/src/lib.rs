// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

// Forbid unwrap and expect to prevent panics
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod api;
pub mod core;
pub mod models;
pub mod utils;

// Re-export commonly used types for convenience
pub use core::claude::{
    ClaudeManager, ClaudeMessage, ClaudeSDK, ClaudeSetup, QueryOptions, QueryRequest,
};

pub use utils::AppError;
