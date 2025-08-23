// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

pub mod claude;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod models;
pub mod state;
pub mod middleware;

// Re-export commonly used types for convenience
pub use claude::{
    ClaudeManager,
    ClaudeSDK,
    ClaudeSetup,
    ClaudeMessage,
    QueryOptions,
    QueryRequest,
};

pub use error::AppError;