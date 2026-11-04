#![cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]

extern crate spiralcore_mvm_live as installed_supervisor_harness;

// Run the same contract against the public production crate in host CI.
include!("installed-supervisor-harness/tests/resources.rs");
