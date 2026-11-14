//! Produce the signed role for the installed FlowMux witness on x86-64 Linux.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "installed_flowmux_fixture/package.rs"]
mod package;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "installed_flowmux_fixture/stage.rs"]
mod stage;

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
use package::verify_sidecar;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::{net::Ipv4Addr, path::Path};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn produce(elf: &Path, digest: &str, address: &str, output: &Path) -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    let ip: Ipv4Addr = address.parse().context("expected canonical IPv4")?;
    ensure!(ip.to_string() == address, "expected canonical IPv4");
    ensure!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "expected lowercase SHA-256"
    );
    let parent = stage::Destination::fresh(output)?;
    let work = stage::scratch(parent.path())?;
    let tree = work.path().join("tree");
    stage::install_guest(elf, digest, &tree)?;
    let packet = stage::scratch(parent.path())?;
    package::build(&tree, ip, work.path(), packet.path())?;
    parent.publish(packet)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    let mut args = std::env::args_os().skip(1);
    let usage = "expected ELF SHA256 IPv4 NEW_OUTPUT";
    let elf = args.next().context(usage)?;
    let digest = args.next().context(usage)?;
    let ip = args.next().context(usage)?;
    let output = args.next().context(usage)?;
    ensure!(args.next().is_none(), "{usage}");
    produce(
        Path::new(&elf),
        digest.to_str().context("expected lowercase SHA-256")?,
        ip.to_str().context("expected canonical IPv4")?,
        Path::new(&output),
    )
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "fixture producer requires x86-64 Linux",
    ))
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
#[path = "installed_flowmux_fixture/tests.rs"]
mod tests;
