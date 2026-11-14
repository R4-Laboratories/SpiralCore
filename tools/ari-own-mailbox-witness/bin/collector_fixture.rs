//! Signed, deny-all role fixtures for the original installed journal witness.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "collector_fixture/package.rs"]
mod package;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "collector_fixture/stage.rs"]
mod stage;

#[derive(Clone, Copy, Debug)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
enum Role {
    Source,
    Destination,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Role {
    fn machine(self) -> &'static str {
        match self {
            Self::Source => "sc-journal-a",
            Self::Destination => "sc-journal-b",
        }
    }
    fn pack(self) -> &'static str {
        match self {
            Self::Source => "spiralcore/sc-journal-a",
            Self::Destination => "spiralcore/sc-journal-b",
        }
    }
    fn guest(self) -> &'static str {
        match self {
            Self::Source => "/usr/local/bin/spiralcore-durable-source",
            Self::Destination => "/usr/local/bin/spiralcore-durable-destination",
        }
    }
    fn module(self) -> &'static str {
        match self {
            Self::Source => "journal-source",
            Self::Destination => "journal-destination",
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn produce(
    source: &std::path::Path,
    source_sha: &str,
    destination: &std::path::Path,
    destination_sha: &str,
    output: &std::path::Path,
) -> anyhow::Result<()> {
    use anyhow::ensure;
    use std::{fs, os::unix::fs::DirBuilderExt};
    for digest in [source_sha, destination_sha] {
        ensure!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "expected lowercase SHA-256"
        );
    }
    let target = stage::Destination::fresh(output)?;
    let packet = stage::scratch(target.path())?;
    let mut roles = Vec::with_capacity(2);
    for (role, elf, digest) in [
        (Role::Source, source, source_sha),
        (Role::Destination, destination, destination_sha),
    ] {
        let work = stage::scratch(target.path())?;
        let tree = work.path().join("tree");
        stage::install_guest(elf, digest, &tree, role.guest())?;
        let role_packet = packet.path().join(role.machine());
        fs::DirBuilder::new().mode(0o700).create(&role_packet)?;
        roles.push(package::build(&tree, role, work.path(), &role_packet)?);
        fs::File::open(&role_packet)?.sync_all()?;
    }
    let handoff = serde_json::to_vec_pretty(&serde_json::json!({"roles": roles}))?;
    ensure!(
        handoff.len() <= 256 << 10,
        "collector operator input exceeds bound"
    );
    stage::write_new(&packet.path().join("collector-operator.json"), &handoff)?;
    target.publish(packet)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use std::path::Path;
    let mut args = std::env::args_os().skip(1);
    let usage = "expected SOURCE_ELF SOURCE_SHA256 DESTINATION_ELF DESTINATION_SHA256 NEW_OUTPUT";
    let source = args.next().context(usage)?;
    let source_sha = args.next().context(usage)?;
    let destination = args.next().context(usage)?;
    let destination_sha = args.next().context(usage)?;
    let output = args.next().context(usage)?;
    ensure!(args.next().is_none(), "{usage}");
    produce(
        Path::new(&source),
        source_sha.to_str().context("expected lowercase SHA-256")?,
        Path::new(&destination),
        destination_sha
            .to_str()
            .context("expected lowercase SHA-256")?,
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
#[path = "collector_fixture/tests.rs"]
mod tests;
