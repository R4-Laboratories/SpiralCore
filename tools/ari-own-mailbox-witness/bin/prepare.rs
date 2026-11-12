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
    let version = "0.28.0-rc.1";
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
