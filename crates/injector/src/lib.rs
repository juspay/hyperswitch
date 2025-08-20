#[cfg(feature = "v2")]
pub mod injector;

// Re-export for v2 feature
#[cfg(feature = "v2")]
pub use injector::*;