use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use mvm_bundler::{BootAssetsInputs, BundleExportInputs, PostureInputs, export_bundle_with_signer};
use mvm_client::{bundle::HostBundleSigner, launch::bundle_runtime::attach_if_installed_bundle};
use mvm_core::{
    arch::GuestArch,
    image_set::{ArtifactName, ImageSetVerification, image_train_lock, verify_image_set_artifacts},
    plan::{
        FsTrustStore,
        bundle::{ArtifactRole, BundleRegistry, verify_bundle_file},
    },
    security::AgentProfile,
    vm_backend::VmStartConfig,
};
use mvm_runtime::driver::{BlockDev, ConsoleCapture, KernelImage, VmmSpec, fc::fc_config_api_puts};

fn text(path: &Path) -> Result<&str> {
    path.to_str().context("fixture path must be UTF-8")
}

fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("isolated asset root required")?,
    );
    ensure!(root.is_absolute() && root.canonicalize()? == root);
    let home = root.join("mvm");
    ensure!(
        mvm_core::config::mvm_home_strict()? == home,
        "isolated MVM_HOME required"
    );
    let runner_home = PathBuf::from(std::env::var_os("HOME").context("runner home missing")?);
    ensure!(root.parent() == Some(runner_home.canonicalize()?.as_path()));
    ensure!(GuestArch::host().to_string() == "x86_64");
    let source = root.join("official");
    let manifest_bytes = fs::read(source.join("image-set.json"))?;
    let signature = fs::read(source.join("image-set.json.bundle"))?;
    let protocols = mvm_build::stage0_kernel::current_image_set_protocol_support();
    let request = ImageSetVerification::new(
        &manifest_bytes,
        &signature,
        &image_train_lock().image_set,
        &source,
    )
    .with_workload_protocols(&protocols);
    let asset_names = [
        "default-microvm-rootfs-x86_64.ext4",
        "default-microvm-rootfs-x86_64.verity",
        "default-microvm-rootfs-x86_64.roothash",
        "default-microvm-vmlinux-x86_64",
        "initramfs-x86_64.tar.gz",
        "runtime-overlay-x86_64.tar.gz",
    ];
    let selected: Vec<_> = asset_names
        .iter()
        .map(|name| ArtifactName::new(*name))
        .collect::<std::result::Result<_, _>>()?;
    let official = verify_image_set_artifacts(&request, &selected)
        .context("verify pinned official publisher and exact selected image assets")?;
    ensure!(official.artifacts.len() == selected.len());
    let actual = |name: &str| -> Result<&Path> {
        official
            .artifacts
            .iter()
            .find(|a| a.name.as_str() == name)
            .map(|asset| asset.path.as_path())
            .context("verified asset missing")
    };

    let stage = root.join("workload");
    fs::create_dir(&stage)?;
    mvm_core::private_fs::set_mode(&stage, 0o700)?;
    let initramfs_stage = stage.join("ramdisk");
    fs::create_dir(&initramfs_stage)?;
    mvm_core::private_fs::set_mode(&initramfs_stage, 0o700)?;
    let initramfs_archive = actual("initramfs-x86_64.tar.gz")?;
    let initrd = mvm_build::initramfs::parse_authenticated_legacy_initramfs(
        initramfs_archive,
        &initramfs_stage,
    )?;
    let workload = stage.join("rootfs.ext4");
    fs::copy(actual("default-microvm-rootfs-x86_64.ext4")?, &workload)?;
    let sidecar = stage.join(mvm_build::builder_vm::SIDECAR_FILENAME);
    // The sealed release's adjacent metadata is a fixture-declared input.
    // Its release-inventory digest is exact, but the signed root does not list
    // it. The isolated local archive signer attests this sidecar separately.
    let fixture_sidecar = source.join("default-microvm-meta-x86_64.json");
    ensure!(
        mvm_core::crypto::image_verify::sha256_file(&fixture_sidecar)?
            == "ec7413b94e3decb890ef3fa959e527be5c1bf437e20fdf5ea80aa6b038987032"
    );
    fs::copy(fixture_sidecar, &sidecar)?;
    let kernel = actual("default-microvm-vmlinux-x86_64")?;
    let tree = fs::read(actual("default-microvm-rootfs-x86_64.verity")?)?;
    let roothash = fs::read_to_string(actual("default-microvm-rootfs-x86_64.roothash")?)?;
    let archive = root.join("installed-native-witness.mvmpkg");
    let mut inputs = BundleExportInputs::new(text(kernel)?, text(&workload)?, "x86_64", &archive)
        .initrd(text(&initrd.image_path)?)
        .verity(&tree, roothash.trim_end())
        .resources(1, 1024)
        .posture(PostureInputs::new(AgentProfile::Dev).allows_egress(true))
        .label("spiralcore-installed-native-fixture")
        .local_build();
    inputs.boot_assets = Some(BootAssetsInputs {
        manifest_bytes: &manifest_bytes,
        manifest_sha256: &official.manifest_sha256,
        runtime_overlay: actual("runtime-overlay-x86_64.tar.gz")?,
        initramfs: initramfs_archive,
    });
    let signer = HostBundleSigner::load_at(&home.join("keys"))?;
    let exported = export_bundle_with_signer(&inputs, &signer)?;
    let trust = FsTrustStore::default_path()?;
    mvm_core::config::create_private_dir(trust.root())?;
    let key = mvm_client::bundle::BundleSigner::verifying_key(&signer);
    mvm_core::util::atomic_io::write_private_new(
        &trust.root().join(format!("{}.pub", exported.key_id.0)),
        key.as_bytes(),
    )?;
    let verified = verify_bundle_file(&archive, &trust)?;
    let registry = BundleRegistry::default_path()?;
    let installed = registry.install_file(&archive, &trust, false)?;
    ensure!(verified.manifest == installed.manifest);
    let selected_path = |role: ArtifactRole| -> Result<PathBuf> {
        let artifact = installed
            .manifest
            .find_by_role(&role)
            .context("installed artifact role missing")?;
        Ok(installed.root.join(&artifact.path))
    };
    let kernel = selected_path(ArtifactRole::Kernel)?;
    let rootfs = selected_path(ArtifactRole::Rootfs)?;
    let mut cfg = VmStartConfig {
        name: "spiralcore-native-witness".into(),
        rootfs_path: text(&rootfs)?.into(),
        kernel_path: Some(text(&kernel)?.into()),
        cpus: 1,
        memory_mib: 1024,
        ..Default::default()
    };
    ensure!(attach_if_installed_bundle(&mut cfg, "firecracker")?);
    ensure!(cfg.legacy_portable_runtime.is_some() && cfg.bundle_boot_assets.is_some());
    ensure!(!cfg.unsigned_runtime_development && cfg.runtime_release_proof.is_none());
    let checked = mvm_runtime::runtime_evidence::verify_runtime_for_launch(
        &cfg,
        mvm_runtime::runtime_evidence::MIN_GUEST_RPC_PROTOCOL_VERSION,
    )?;
    ensure!(checked.is_some(), "native fixture has no verified runtime");
    let mut blocks = Vec::new();
    for path in [
        Some(cfg.rootfs_path.as_str()),
        cfg.verity_path.as_deref(),
        cfg.runtime_overlay_path.as_deref(),
        cfg.runtime_overlay_verity_path.as_deref(),
    ] {
        let source = PathBuf::from(path.context("verified boot block missing")?);
        blocks.push(BlockDev {
            source,
            read_only: true,
            ephemeral: false,
            slot: blocks.len().try_into()?,
        });
    }
    let spec = VmmSpec {
        name: cfg.name.clone(),
        kernel: KernelImage::Path(kernel.clone()),
        initramfs: cfg.initrd_path.as_ref().map(PathBuf::from),
        cmdline: String::new(),
        vcpus: 1,
        cpu_grant: None,
        memory_mib: 1024,
        mem_initial_mib: None,
        blocks,
        vsock: Vec::new(),
        console: ConsoleCapture {
            log_path: root.join("not-booted-console"),
        },
        shares: Vec::new(),
        trusted_builder: false,
        builder_egress_endpoint: None,
        plan_binding: None,
    };
    let api = fc_config_api_puts(
        &spec,
        text(&kernel)?,
        text(&root.join("unbound-vsock"))?,
        text(&root)?,
        Default::default(),
    );
    let paths: Vec<_> = api.iter().map(|put| put.path.as_str()).collect();
    ensure!(
        paths.contains(&"/vsock") && !paths.iter().any(|path| path.contains("network-interface"))
    );
    let actual_paths = [
        Path::new(&cfg.rootfs_path),
        kernel.as_path(),
        Path::new(
            cfg.initrd_path
                .as_deref()
                .context("installed initrd missing")?,
        ),
        Path::new(
            cfg.runtime_overlay_path
                .as_deref()
                .context("installed overlay missing")?,
        ),
        Path::new(
            cfg.runtime_overlay_verity_path
                .as_deref()
                .context("installed overlay verity missing")?,
        ),
        Path::new(
            cfg.verity_path
                .as_deref()
                .context("installed workload verity missing")?,
        ),
    ];
    let rows: Vec<_> = actual_paths
        .into_iter()
        .map(|path| -> Result<_> {
            Ok(serde_json::json!({
                "path": path,
                "bytes": fs::metadata(path)?.len(),
                "sha256": mvm_core::crypto::image_verify::sha256_file(path)?,
            }))
        })
        .collect::<Result<_>>()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "bundle": archive,
            "signed_plan_pin": verified.plan_artifact(),
            "bundle_schema": verified.manifest.schema_version,
            "registry_root": installed.root,
            "publisher_image_set": official.manifest_sha256.as_str(),
        "metadata_scope": "adjacent release digest; locally attested, absent from signed image-set root",
            "resolver": "mvm_client::launch::bundle_runtime::attach_if_installed_bundle/firecracker",
            "selected": rows,
            "firecracker_preflight_put_paths": paths,
            "vcpus": 1, "memory_mib": 1024, "executed_firecracker": false,
            "wire_policy": "HostVsockProxy to be bound by original signed plan, no NIC",
        }))?
    );
    Ok(())
}
