#[cfg(target_os = "linux")]
#[path = "../installer.rs"]
mod implementation;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    implementation::run()
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("This witness requires a native Linux host.");
    std::process::ExitCode::FAILURE
}
