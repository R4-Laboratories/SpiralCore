use anyhow::{Context, Result, ensure};
use mvm_build::{
    initramfs::parse_authenticated_legacy_initramfs,
    runtime_overlay::parse_authenticated_legacy_overlay,
};
use mvm_core::{arch::GuestArch, crypto::image_verify::sha256_file};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(
        args.next()
            .context("missing exact portable input directory")?,
    )
    .canonicalize()?;
    let source =
        PathBuf::from(args.next().context("missing checked runtime cache")?).canonicalize()?;
    ensure!(args.next().is_none());
    let readback = input.join("readback");
    let overlay_stage = readback.join("overlay");
    let initramfs_stage = readback.join("initramfs");
    let refused_stage = readback.join("negative");
    for stage in [&overlay_stage, &initramfs_stage, &refused_stage] {
        ensure!(std::fs::read_dir(stage)?.next().is_none());
    }
    let overlay = parse_authenticated_legacy_overlay(
        &input.join("runtime-overlay-x86_64.tar.gz"),
        &overlay_stage,
        GuestArch::X86_64,
    )?;
    let initramfs = parse_authenticated_legacy_initramfs(
        &input.join("initramfs-x86_64.tar.gz"),
        &initramfs_stage,
    )?;
    for (name, decoded, original) in [
        (
            "overlay",
            overlay_stage.join("overlay.ext4"),
            source.join("runtime-overlay/0.26.0/x86_64/overlay.ext4"),
        ),
        (
            "overlay verity",
            overlay_stage.join("overlay.verity"),
            source.join("runtime-overlay/0.26.0/x86_64/overlay.verity"),
        ),
        (
            "initramfs",
            initramfs.image_path,
            source.join("initramfs/0.26.0/x86_64/initramfs.cpio.gz"),
        ),
    ] {
        ensure!(
            std::fs::read(&decoded)? == std::fs::read(&original)?,
            "portable {name} readback differs"
        );
        println!("{name} exact SHA-256 {}", sha256_file(&decoded)?);
    }
    ensure!(
        overlay.roothash
            == std::fs::read_to_string(
                source.join("runtime-overlay/0.26.0/x86_64/overlay.roothash")
            )?
            .trim()
    );
    let refusal = parse_authenticated_legacy_initramfs(
        &input.join("initramfs-tampered.tar.gz"),
        &refused_stage,
    )
    .expect_err("modified payload must not match the original checksum");
    ensure!(matches!(
        refusal,
        mvm_build::initramfs::InitramfsBuildError::ChecksumMismatch { .. }
    ));
    println!("tampered initramfs refused: {refusal}");
    Ok(())
}
