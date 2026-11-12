//! Host-only qualification builder. The only incoming guest data is one
//! executable per role, verified against the supplied operator digest.
//! Compile against the matching MVM build/agent/core and serde_json crates.
use std::error::Error;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use mvm_agentd::runtime_config::{
    self, ConcurrencyConfig, InProcessMode, RuntimeConfig, WarmProcessConfig,
};
use mvm_build::builder_vm::GuestSidecar;
use mvm_build::oci_runtime_inject::{ImageRuntimeConfig, MvmRuntimeBinaries, inject_mvm_runtime};
use mvm_build::rootfs::{MaterializeExt4Input, materialize_ext4_rejecting_unsupported};
use mvm_build::run_image::unpacked_tree_size;
use mvm_core::crypto::image_verify::sha256_file;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [
        role,
        program,
        expected_hex,
        runtime_bin_dir,
        wrapper_hex,
        work,
    ] = args.as_slice()
    else {
        return Err("usage: ROLE PROGRAM TRUSTED_PROGRAM_SHA256 RUNTIME_BIN_DIR TRUSTED_WRAPPER_SHA256 FRESH_WORK_DIR".into());
    };
    let (machine, basename) = match role.as_str() {
        "a" => ("spiralcore-cast-alpha", "spiralcore-mailbox-producer"),
        "b" => ("spiralcore-cast-beta", "spiralcore-cast-recipient"),
        _ => return Err("unknown addressed role".into()),
    };
    let work = Path::new(work);
    fs::DirBuilder::new().mode(0o700).create(work)?;
    let tree = work.join("stage");
    let output = work.join("image");
    fs::create_dir_all(tree.join("usr/local/bin"))?;
    fs::create_dir(&output)?;
    let guest_path = format!("/usr/local/bin/{basename}");
    let staged_program = tree.join(&guest_path[1..]);
    fs::copy(program, &staged_program)?;
    fs::set_permissions(&staged_program, fs::Permissions::from_mode(0o555))?;
    if sha256_file(&staged_program)? != *expected_hex {
        return Err("staged role executable digest mismatch".into());
    }

    // Negative control. The official injector must discard every input
    // file below its runtime-owned root, even if named like a valid policy.
    let cfg = tree.join("etc/mvm");
    fs::create_dir_all(&cfg)?;
    for name in [
        "runtime.json",
        "entrypoint",
        "image-runtime.json",
        "untrusted-marker",
    ] {
        fs::write(cfg.join(name), "UNTRUSTED FIXTURE CONTENT")?;
    }
    let bin = PathBuf::from(runtime_bin_dir);
    let binaries = MvmRuntimeBinaries {
        agent: bin.join("mvm-guest-agent"),
        netinit: bin.join("mvm-guest-netinit"),
        egress_client: bin.join("mvm-egress-client"),
        entrypoint_runner: bin.join("mvm-oci-entrypoint"),
    };
    let entry = ImageRuntimeConfig {
        argv: vec![guest_path.clone()],
        env: vec![],
        working_dir: Some("/".into()),
    };
    inject_mvm_runtime(&tree, &binaries, Some(&entry), true)?;
    if cfg.join("runtime.json").exists()
        || cfg.join("untrusted-marker").exists()
        || fs::read_to_string(cfg.join("entrypoint"))?.trim()
            != "/usr/lib/mvm/wrappers/oci-entrypoint"
        || serde_json::from_slice::<ImageRuntimeConfig>(&fs::read(cfg.join("image-runtime.json"))?)?
            != entry
        || sha256_file(&tree.join("usr/lib/mvm/wrappers/oci-entrypoint"))? != *wrapper_hex
    {
        return Err("input controls survived or admitted OCI configuration differs".into());
    }
    // This host generates exactly one serial worker from its selected role.
    // Never parse any application-supplied runtime configuration.
    let worker = RuntimeConfig {
        language: "rust".into(),
        module: "cast".into(),
        function: "main".into(),
        format: "binary".into(),
        source_path: guest_path,
        concurrency: Some(ConcurrencyConfig::WarmProcess(WarmProcessConfig {
            max_calls_per_worker: 100,
            max_rss_mb: 64,
            pool_size: 1,
            in_process: InProcessMode::Serial,
            max_queue_depth: Some(1),
        })),
    };
    let policy = cfg.join("runtime.json");
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&policy)?;
    serde_json::to_writer(&mut out, &worker)?;
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o444))?;
    out.sync_all()?;
    if runtime_config::load_from(&policy)?.as_ref() != Some(&worker) {
        return Err("guest runtime parser rejected the operator policy".into());
    }
    let image = output.join("rootfs.ext4");
    let spec = MaterializeExt4Input::new(tree.clone(), image.clone(), unpacked_tree_size(&tree)?)
        .with_verity();
    materialize_ext4_rejecting_unsupported(&spec)?;
    GuestSidecar::for_oci_run(machine, true, true)
        .with_entrypoint_argv(entry.argv)
        .with_libc(mvm_build::guest_libc::detect_guest_libc(&tree))
        .write_to_dir(&output)?;
    println!("{} {}", sha256_file(&image)?, image.display());
    Ok(())
}
