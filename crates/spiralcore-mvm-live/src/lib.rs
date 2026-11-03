//! Explicit development-only real-guest integration witness.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub mod campaign;
pub mod config;
pub mod fixtures;
pub mod guest;
pub mod host_budget;

#[cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]
pub mod installed_owner;

#[cfg(any(all(target_os = "linux", feature = "linux-host"), all(test, target_os = "macos")))]
pub mod linux_host;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod host;

#[cfg(all(target_os = "linux", feature = "linux-host"))]
pub use linux_host as host;

#[cfg(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "linux", feature = "linux-host")
))]
pub mod orchestrator;
