//! Explicit local fixture signing initialization and pinned bundle installation.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "collector_install/install.rs"]
mod install;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    let mut args = std::env::args_os().skip(1);
    match args.next().as_deref().and_then(|s| s.to_str()) {
        Some("init") => {
            ensure!(args.next().is_none(), "expected init");
            install::initialize()
        }
        Some("install") => {
            let packet = args
                .next()
                .context("expected install PACKET EXPECTED_PUBLISHER_HEX")?;
            let publisher = args.next().context("expected external publisher pin")?;
            ensure!(
                args.next().is_none(),
                "expected install PACKET EXPECTED_PUBLISHER_HEX"
            );
            install::install(
                std::path::Path::new(&packet),
                publisher.to_str().context("publisher pin must be UTF-8")?,
            )
        }
        _ => anyhow::bail!("expected init or install PACKET EXPECTED_PUBLISHER_HEX"),
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "fixture installer requires x86-64 Linux",
    ))
}
