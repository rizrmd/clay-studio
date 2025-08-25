// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

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