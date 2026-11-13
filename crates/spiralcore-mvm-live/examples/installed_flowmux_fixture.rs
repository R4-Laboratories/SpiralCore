//! Build the signed role consumed by the installed FlowMux qualification.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn produce(_: &std::path::Path, _: &str, _: &str, _: &std::path::Path) -> anyhow::Result<()> {
    anyhow::bail!("fixture producer absent")
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn verify_sidecar(_: &std::path::Path, _: &[String]) -> anyhow::Result<()> {
    anyhow::bail!("sidecar check absent")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err(std::io::Error::other("fixture producer is not implemented").into())
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "installed_flowmux_fixture/tests.rs"]
mod tests;
