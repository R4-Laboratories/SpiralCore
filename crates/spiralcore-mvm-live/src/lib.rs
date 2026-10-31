//! Explicit development-only real-guest integration witness.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub mod campaign;
pub mod config;
pub mod fixtures;
pub mod guest;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod host;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod orchestrator;
