//! Private qualification entrypoint for direct launch by the guest agent.
fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("installed-mailbox-witness"))
        || args.next().is_some()
    {
        std::process::exit(29);
    }
    // This checks the effective filter before opening credentials or dialing.
    // The witness itself has no path that installs a filter.
    if let Err(code) = spiralcore_mvm_live::mailbox_witness::run_guest() {
        std::process::exit(i32::from(code));
    }
}
