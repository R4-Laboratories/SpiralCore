//! Synthetic Firecracker witness entrypoint for a legacy guest agent.
//! This file is built only into the private development fixture.
use mvm_agentd::workload_seccomp;
use mvm_core::crypto::seccomp::SeccompTier;
use std::{fs::File, io::Read as _};

fn run() -> Result<(), u8> {
    let mut arguments = std::env::args_os();
    if arguments.nth(1).as_deref() != Some(std::ffi::OsStr::new("installed-mailbox-witness"))
        || arguments.next().is_some()
    {
        return Err(29);
    }
    let mut cmdline = String::new();
    File::open("/proc/cmdline")
        .map_err(|_| 30_u8)?
        .take(4097)
        .read_to_string(&mut cmdline)
        .map_err(|_| 30_u8)?;
    if cmdline.len() > 4096
        || workload_seccomp::tier_from_cmdline(&cmdline) != Ok(Some(SeccompTier::Vsock))
    {
        return Err(31);
    }
    let program = workload_seccomp::compile(SeccompTier::Vsock)
        .map_err(|_| 32_u8)?
        .ok_or(32_u8)?;
    workload_seccomp::install(&program).map_err(|_| 33_u8)?;
    // The normal witness still runs its effective filter check before it
    // opens credentials or dials. Only the source of confinement differs.
    spiralcore_mvm_live::mailbox_witness::run_guest()
}

fn main() {
    if let Err(code) = run() {
        std::process::exit(i32::from(code));
    }
}
