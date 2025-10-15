// Clay Studio Backend Library
// Exposes the Claude SDK and other modules for external use

// Warn on unwrap usage to encourage proper error handling
// But allow it in tests and for known safe operations
#![allow(clippy::unwrap_used)]
// Allow expect in tests and regex patterns where it's appropriate
#![allow(clippy::expect_used)]
// Allow reasonable warnings for development
#![allow(clippy::redundant_closure)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::useless_format)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::len_zero)]
#![allow(clippy::new_without_default)]
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
