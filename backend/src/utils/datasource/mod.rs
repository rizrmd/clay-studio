// Core abstractions and interfaces
pub mod core;

// Database-specific connector implementations  
pub mod connectors;

// Connection pooling logic
pub mod pooling;

// Shared utilities
pub mod common;

// Test modules
#[cfg(test)]
pub mod tests;

// Re-export commonly used items
pub use core::*;
// Removed unused import - uncomment when needed
// pub use connectors::*;
pub use pooling::*;
