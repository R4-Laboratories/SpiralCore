//! One native SDK fixture using the retained installed service source.
#![cfg(target_os = "linux")]

pub mod host_budget;
pub mod installed_disk;
pub mod installed_owner;
pub mod installed_parent;
pub mod installed_resources;
pub(crate) mod installed_retained;
pub mod installed_supervisor;
pub(crate) mod installed_worker;
