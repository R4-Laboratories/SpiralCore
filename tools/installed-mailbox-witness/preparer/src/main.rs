#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf, time::Instant};

    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--provision-only" {
        let home = PathBuf::from(&args[1]);
        ensure!(!home.try_exists()?, "fresh private home required");
        mvm_vmm::host::host_agent_installation::create_new_home_and_provision(&home, "local")?
            .verify()?;
        return Ok(());
    }
    ensure!(
        args.len() == 3,
        "usage: spiralcore-mvm-prepare-runtime SOURCE_ROOT PRIVATE_MVM_HOME REPORT_JSON"
    );
    let source = PathBuf::from(&args[0]);
    let home = PathBuf::from(&args[1]);
    let report = PathBuf::from(&args[2]);
    ensure!(
        source.is_absolute() && home.is_absolute() && report.is_absolute(),
        "absolute paths required"
    );
    let parent = fs::canonicalize(report.parent().context("report parent required")?)?;
    ensure!(
        !parent.starts_with(&home)
            && !fs::canonicalize(&home).is_ok_and(|root| parent.starts_with(root)),
        "report must remain outside MVM_HOME"
    );
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&report)?;
    let started = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let result = (|| -> anyhow::Result<serde_json::Value> {
        // A previous runtime-only attempt may have provisioned this isolated home.
        // Open and verify it exactly as canonical installation does. Never replace it.
        let preparation = mvm_vmm::host::host_agent_installation::prepare_home(&home, "local")?;
        preparation.verify()?;
        mvm_vmm::host::host_agent_installation::descriptor(&home, "local")?;
        ensure!(home.is_dir(), "MVM_HOME must be a private 0700 directory");
        ensure!(
            fs::canonicalize(&home)? == home,
            "canonical MVM_HOME required"
        );
        ensure!(
            std::env::var_os("MVM_HOME").as_deref() == Some(home.as_os_str()),
            "MVM_HOME must equal the explicit private home"
        );
        match fs::symlink_metadata(mvm_core::config::vms_dir_at(&home)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => anyhow::bail!("preparation requires a fresh VM namespace"),
        }
        let source = fs::canonicalize(&source)?;
        ensure!(
            source.join("Cargo.toml").is_file(),
            "source Cargo.toml missing"
        );
        let cache = PathBuf::from(mvm_core::config::mvm_cache_dir());
        let arch = mvm_core::arch::GuestArch::host();
        // This version is bound to the exact dependency revision in Cargo.toml.
        let version = "0.28.0-rc.1";
        let runtime = mvm_build::guest_runtime::resolve_or_build_source_guest_runtime(
            &cache, version, arch, &source,
        )?;
        let pieces =
            mvm_build::runtime_pieces::assemble_runtime_pieces(&cache, version, arch, &runtime)?;
        let sdk: Vec<_> = pieces
            .sdk_sidecars
            .iter()
            .map(|piece| {
                serde_json::json!({
                    "image": piece.image, "sha256": piece.image_sha256, "libc": piece.libc,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "status": "prepared", "native_transfer": "NOT_RUN",
            "mvm_api_revision": "f780207089dbc1c90876c44d54e7221b6d79108d",
            "source_checkout": source, "runtime_archive_sha256": runtime.digest,
            "runtime_manifest": runtime.manifest, "runtime_root": runtime.root,
            "arch": arch.to_string(), "version": version,
            "overlay": {"image": pieces.runtime_overlay.overlay_ext4,
                "verity": pieces.runtime_overlay.sidecar, "roothash": pieces.runtime_overlay.roothash},
            "initramfs": {"image": pieces.initramfs.image_path,
                "hash_record": fs::read_to_string(&pieces.initramfs.hash_path)?},
            "sdk_sidecars": sdk,
        }))
    })();
    let mut record = match result {
        Ok(record) => record,
        Err(error) => {
            serde_json::json!({"status": "failed_preparation", "native_transfer": "NOT_RUN", "error": format!("{error:#}")})
        }
    };
    record["preparation_wall_ns"] = serde_json::json!(started.elapsed().as_nanos());
    record["preparation_process_cpu_ns"] = serde_json::json!(cpu.elapsed().as_nanos());
    record["cpu_scope"] = serde_json::json!("this process only; compiler children excluded");
    serde_json::to_writer_pretty(&mut output, &record)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    ensure!(
        record["status"] == "prepared",
        "runtime preparation failed; inspect retained JSON"
    );
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("this preparation tool requires the authorized Linux test host")
}
