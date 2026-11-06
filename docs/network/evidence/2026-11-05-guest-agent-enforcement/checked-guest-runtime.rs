use mvm_build::{guest_runtime, runtime_pieces};
use mvm_core::arch::GuestArch;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().ok_or("missing SDK root")?).canonicalize()?;
    let cache = PathBuf::from(args.next().ok_or("missing isolated cache")?).canonicalize()?;
    if args.next().is_some() || !root.join("Cargo.lock").is_file() || root == cache {
        return Err("invalid checked-runtime inputs".into());
    }
    let version = env!("CARGO_PKG_VERSION");
    let arch = GuestArch::host();
    let runtime =
        guest_runtime::resolve_or_build_source_guest_runtime(&cache, version, arch, &root)?;
    let pieces = runtime_pieces::assemble_runtime_pieces(&cache, version, arch, &runtime)?;
    let checked = guest_runtime::source::check_source_runtime(&cache, version, arch)?;
    if checked.archive_sha256() != runtime.digest || runtime.release_proof.is_some() {
        return Err("runtime provenance differs from checked source archive".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "version": version,
            "arch": arch.to_string(),
            "archive_sha256": checked.archive_sha256(),
            "runtime_root": runtime.root,
            "overlay_ext4": pieces.runtime_overlay.overlay_ext4,
            "overlay_verity": pieces.runtime_overlay.sidecar,
            "overlay_root_hash": pieces.runtime_overlay.roothash,
            "initramfs": pieces.initramfs.image_path,
            "sdk_sidecar_count": pieces.sdk_sidecars.len(),
        })
    );
    Ok(())
}
