// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

// Forbid unwrap and expect to prevent panics
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod core;
pub mod utils;
pub mod api;
pub mod models;

// Re-export commonly used types for convenience
pub use core::claude::{
    ClaudeManager,
    ClaudeSDK,
    ClaudeSetup,
    ClaudeMessage,
    QueryOptions,
    QueryRequest,
};

pub use utils::AppError;