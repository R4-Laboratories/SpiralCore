use anyhow::{Context, Result, ensure};
use mvm_core::arch::GuestArch;
use std::{env, ffi::OsString, path::PathBuf};

fn inputs(mut args: impl Iterator<Item = OsString>) -> Result<(PathBuf, GuestArch)> {
    let source = args.next().context("missing pinned source")?;
    let arch = match args.next() {
        None => GuestArch::host(),
        Some(value) => value
            .to_str()
            .context("invalid guest architecture")?
            .parse()?,
    };
    ensure!(
        args.next().is_none(),
        "expected prepare SOURCE [GUEST_ARCH]"
    );
    Ok((PathBuf::from(source), arch))
}

fn workspace_version(manifest: &str) -> Result<String> {
    manifest
        .parse::<toml::Table>()?
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .context("missing MVM workspace package version")
}

fn matched_workspace_version(source_manifest: &str) -> Result<String> {
    // Cargo.toml declares each linked MVM crate under ../source; from bin/
    // that same workspace manifest is ../../source/Cargo.toml.
    let linked = workspace_version(include_str!("../../source/Cargo.toml"))?;
    ensure!(
        workspace_version(source_manifest)? == linked,
        "guest source version differs from the linked host"
    );
    Ok(linked)
}

fn main() -> Result<()> {
    let (source, arch) = inputs(env::args_os().skip(1))?;
    let source = source.canonicalize()?;
    let home = PathBuf::from(env::var_os("MVM_HOME").context("MVM_HOME")?);
    ensure!(
        home.is_absolute() && home.canonicalize()? == home && !home.join("vms").exists(),
        "isolated unoccupied native home required"
    );
    let config = mvm_core::user_config::load_strict(None)?;
    ensure!(
        config.max_mailbox_retention_ms.is_none(),
        "existing host retention configuration"
    );
    mvm_core::user_config::save(
        &mvm_core::user_config::MvmConfig {
            max_mailbox_retention_ms: Some(30_000),
            ..config
        },
        None,
    )?;
    let cache = PathBuf::from(mvm_core::config::mvm_cache_dir());
    ensure!(cache.starts_with(&home), "shared default cache refused");
    let version = matched_workspace_version(&std::fs::read_to_string(source.join("Cargo.toml"))?)?;
    let version = version.as_str();
    let runtime = mvm_build::guest_runtime::resolve_or_build_source_guest_runtime(
        &cache, version, arch, &source,
    )?;
    let pieces =
        mvm_build::runtime_pieces::assemble_runtime_pieces(&cache, version, arch, &runtime)?;
    let sha = |p: &std::path::Path| mvm_core::crypto::image_verify::sha256_file(p);
    println!(
        "{}",
        serde_json::json!({"mvm_version":version,"arch":arch.to_string(),"guest_archive_sha256":runtime.digest,
       "initramfs_sha256":sha(&pieces.initramfs.image_path)?,"initramfs_hash_record_sha256":sha(&pieces.initramfs.hash_path)?,
       "overlay_sha256":sha(&pieces.runtime_overlay.overlay_ext4)?,"overlay_verity_sha256":sha(&pieces.runtime_overlay.sidecar)?,
       "overlay_root_hash":pieces.runtime_overlay.roothash,"sdk_sidecars":pieces.sdk_sidecars.len(),"host_retention_ceiling_ms":30000})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mvm_core::arch::GuestArch;
    #[test]
    fn guest_runtime_uses_the_linked_mvm_workspace_version() {
        let linked = include_str!("../../source/Cargo.toml");
        assert_eq!(
            matched_workspace_version(linked).unwrap(),
            workspace_version(linked).unwrap()
        );
        for invalid in ["", "bad [", "[workspace.package]\nversion=2"] {
            assert!(matched_workspace_version(invalid).is_err());
        }
        let wrong = "[workspace.package]\nversion='0.0.0-unrelated'";
        assert_eq!(workspace_version(wrong).unwrap(), "0.0.0-unrelated");
        assert!(
            matched_workspace_version(wrong)
                .unwrap_err()
                .to_string()
                .contains("differs")
        );
    }

    #[test]
    fn selects_the_guest_architecture_before_preparing_private_state() {
        assert_eq!(
            inputs(["guest-source"].map(Into::into).into_iter()).unwrap(),
            (PathBuf::from("guest-source"), GuestArch::host())
        );
        for (arg, expected) in [
            ("aarch64", GuestArch::Aarch64),
            ("x86_64", GuestArch::X86_64),
        ] {
            assert_eq!(
                inputs(["guest-source", arg].map(Into::into).into_iter())
                    .unwrap()
                    .1,
                expected
            );
        }
        assert!(inputs(["guest-source", "sparc"].map(Into::into).into_iter()).is_err());
        assert!(
            inputs(
                ["guest-source", "aarch64", "unexpected"]
                    .map(Into::into)
                    .into_iter()
            )
            .is_err()
        );
    }
}
