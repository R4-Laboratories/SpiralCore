use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use mvm_bundler::{BootAssetsInputs, BundleExportInputs, PostureInputs, export_bundle_with_signer};
use mvm_client::{bundle::HostBundleSigner, launch::bundle_runtime::attach_if_installed_bundle};
use mvm_core::{
    arch::GuestArch,
    image_set::{
        ArtifactName, GitCommit, ImageSetProducer, ImageSetRequirement, ImageSetVerification,
        ImageSetVersion, LocalCheckouts, ProtocolRange, RepoIdentity, WorktreeState,
        image_train_lock, require_complete, validate_structure, verify_image_set_artifacts,
    },
    packs::Sha256Hex,
    plan::{
        FsTrustStore,
        bundle::TrustStore,
        bundle::{ArtifactRole, BundleRegistry, verify_bundle_file},
    },
    security::AgentProfile,
    vm_backend::VmStartConfig,
};
use mvm_runtime::driver::{BlockDev, ConsoleCapture, KernelImage, VmmSpec, fc::fc_config_api_puts};

fn text(path: &Path) -> Result<&str> {
    path.to_str().context("fixture path must be UTF-8")
}

fn checked(program: &str, args: &[&str]) -> Result<Vec<u8>> {
    let output = std::process::Command::new(program).args(args).output()?;
    ensure!(
        output.status.success(),
        "private fixture image operation failed: {program}"
    );
    Ok(output.stdout)
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
    ensure!(std::env::var_os("HOME").as_deref() == Some(root.join("home").as_os_str()));
    ensure!(GuestArch::host().to_string() == "x86_64");
    let guest = PathBuf::from(
        std::env::args_os()
            .nth(2)
            .context("static mailbox client missing")?,
    );
    let guest_sha = std::env::args()
        .nth(3)
        .context("static mailbox client digest missing")?;
    let workload_uid: u32 = std::env::args()
        .nth(4)
        .context("workload UID missing")?
        .parse()?;
    let workload_gid: u32 = std::env::args()
        .nth(5)
        .context("workload GID missing")?
        .parse()?;
    ensure!(guest.is_absolute() && guest.canonicalize()? == guest);
    ensure!(mvm_core::crypto::image_verify::sha256_file(&guest)? == guest_sha);
    use std::io::Read as _;
    let mut bootstrap = Vec::new();
    std::io::stdin().take(65_537).read_to_end(&mut bootstrap)?;
    ensure!(
        !bootstrap.is_empty() && bootstrap.len() <= 65_536,
        "private mailbox bootstrap size invalid"
    );

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

    let (stage_name, bundle_name) = match std::env::args().nth(6).as_deref() {
        None | Some("1") => ("workload", "installed-mailbox-native-witness.mvmpkg"),
        Some("2") => ("workload-b", "installed-mailbox-native-witness-b.mvmpkg"),
        _ => anyhow::bail!("invalid fixture slot"),
    };
    let stage = root.join(stage_name);
    fs::create_dir(&stage)?;
    mvm_core::private_fs::set_mode(&stage, 0o700)?;
    let initramfs_stage = stage.join("ramdisk");
    fs::create_dir(&initramfs_stage)?;
    mvm_core::private_fs::set_mode(&initramfs_stage, 0o700)?;
    let input = Path::new("/var/tmp/spiralcore-guest-enforcement-20261105/portable-inputs");
    let initramfs_archive = input.join("initramfs-x86_64.tar.gz");
    let overlay_archive = input.join("runtime-overlay-x86_64.tar.gz");
    let archives = [
        (
            &initramfs_archive,
            "74eb04ab25953da8760399ea2e7a33b22f9c51088ba484e86cba6f434dfbba1f",
        ),
        (
            &overlay_archive,
            "69adcbb059043b68cd00f51a31c2f50eb325509bbf2319e449d31d50e391bca5",
        ),
    ];
    let mut manifest = official.manifest.clone();
    ensure!(manifest.schema_version == 2);
    let mvm_commit = GitCommit::new("b55d0c4dac04587c918cb1721971af8a33b495ee")?;
    manifest.producer = ImageSetProducer::LocalCheckouts(LocalCheckouts {
        images: RepoIdentity {
            commit: official.release.source_commit.clone(),
            worktree: WorktreeState::Clean,
        },
        mvm: Some(RepoIdentity {
            commit: mvm_commit.clone(),
            worktree: WorktreeState::Clean,
        }),
    });
    manifest.mvm_source_commit = Some(mvm_commit);
    manifest.set_version = ImageSetVersion::new("0.0.0-spiralcore-local")?;
    manifest.issued_at = chrono::Utc::now();
    manifest.revocation_channel = None;
    manifest.supersedes = None;
    manifest.compatibility.guest_agent_protocol = Some(ProtocolRange::new(
        mvm_runtime::runtime_evidence::MIN_GUEST_RPC_PROTOCOL_VERSION,
        mvm_runtime::runtime_evidence::GUEST_RPC_PROTOCOL_VERSION,
    )?);
    // The other verified image declarations come from the original image
    // checkout. Only these two runtime members came from the composed SDK.
    use mvm_core::plan::bundle::{BOOT_ASSET_INITRAMFS_ROLE, BOOT_ASSET_OVERLAY_ROLE};
    manifest.members.retain(|member| {
        !matches!(
            member.role,
            mvm_core::image_set::ImageSetRole::RetiredGuestRuntime(_)
        ) || (member.target == mvm_core::image_set::MemberTarget::Arch(GuestArch::X86_64)
            && [BOOT_ASSET_OVERLAY_ROLE, BOOT_ASSET_INITRAMFS_ROLE].contains(&member.role))
    });
    for member in &mut manifest.members {
        member.pack_hash = None;
        member.sbom = None;
    }
    for (archive, expected) in archives {
        let hash = mvm_core::crypto::image_verify::sha256_file(archive)?;
        ensure!(hash == expected);
        let name = archive.file_name().context("archive basename")?;
        let selected: Vec<_> = manifest
            .members
            .iter_mut()
            .flat_map(|member| member.artifacts.iter_mut())
            .filter(|artifact| std::ffi::OsStr::new(artifact.name.as_str()) == name)
            .collect();
        let [asset] = selected
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected one artifact for {name:?}"))?;
        asset.sha256 = Sha256Hex::new(hash)?;
        asset.size = fs::metadata(archive)?.len();
    }
    validate_structure(&manifest)?;
    require_complete(&manifest, &ImageSetRequirement::current_train())?;
    let local_manifest_bytes = serde_json::to_vec(&manifest)?;
    let local_manifest_sha =
        Sha256Hex::new(mvm_core::plan::bundle::sha256_hex(&local_manifest_bytes))?;
    let initrd = mvm_build::initramfs::parse_authenticated_legacy_initramfs(
        &initramfs_archive,
        &initramfs_stage,
    )?;
    let workload = stage.join("rootfs.ext4");
    fs::copy(actual("default-microvm-rootfs-x86_64.ext4")?, &workload)?;
    mvm_core::private_fs::set_mode(&workload, 0o600)?;
    fs::OpenOptions::new()
        .write(true)
        .open(&workload)?
        .set_len(14 << 20)?;
    checked("/usr/sbin/resize2fs", &[text(&workload)?])?;
    let private = stage.join("mailbox-bootstrap.input");
    mvm_core::util::atomic_io::write_private_new(&private, &bootstrap)?;
    let guest_path = "/bin/spiralcore-installed-mailbox";
    let credential_path = "/usr/lib/spiralcore/installed-mailbox.json";
    for input in [&guest, &private] {
        ensure!(
            text(input)?
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b)),
            "unsupported path for fixture ext4 writer"
        );
    }
    for request in [
        "mkdir /usr/lib".to_owned(),
        "mkdir /usr/lib/spiralcore".to_owned(),
        format!("write {} {guest_path}", text(&guest)?),
        format!("set_inode_field {guest_path} mode 0100755"),
        format!("write {} {credential_path}", text(&private)?),
        format!("set_inode_field {credential_path} mode 0100600"),
        format!("set_inode_field {credential_path} uid {workload_uid}"),
        format!("set_inode_field {credential_path} gid {workload_gid}"),
    ] {
        checked(
            "/usr/sbin/debugfs",
            &["-w", "-R", &request, text(&workload)?],
        )?;
    }
    let installed_guest = stage.join("mailbox-guest.readback");
    let installed_credential = stage.join("mailbox-bootstrap.readback");
    for (internal, copy) in [
        (guest_path, &installed_guest),
        (credential_path, &installed_credential),
    ] {
        checked(
            "/usr/sbin/debugfs",
            &[
                "-R",
                &format!("dump {internal} {}", text(copy)?),
                text(&workload)?,
            ],
        )?;
    }
    ensure!(mvm_core::crypto::image_verify::sha256_file(&installed_guest)? == guest_sha);
    ensure!(
        fs::read(&installed_credential)? == bootstrap,
        "private bootstrap ext4 readback differs"
    );
    let inspected = checked(
        "/usr/sbin/debugfs",
        &["-R", &format!("stat {credential_path}"), text(&workload)?],
    )?;
    let inode = std::str::from_utf8(&inspected)?;
    ensure!(inode.contains("Type: regular") && inode.contains("Mode:  0600"));
    let fields: Vec<_> = inode.split_whitespace().collect();
    let field = |name| {
        fields
            .iter()
            .position(|value| *value == name)
            .and_then(|index| fields.get(index + 1))
            .and_then(|v| v.parse::<u32>().ok())
    };
    ensure!(
        field("User:") == Some(workload_uid)
            && field("Group:") == Some(workload_gid)
            && field("Links:") == Some(1),
        "private bootstrap inode authority differs"
    );
    for disposable in [&private, &installed_credential, &installed_guest] {
        fs::remove_file(disposable)?;
    }
    checked("/usr/sbin/e2fsck", &["-fn", text(&workload)?])?;
    let private_root_hash = mvm_build::rootfs::emit_verity_sidecars(&workload)?;
    mvm_core::private_fs::set_mode(&workload, 0o400)?;

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
    let tree = fs::read(stage.join("rootfs.verity"))?;
    let roothash = private_root_hash;
    let archive = root.join(bundle_name);
    let mut inputs = BundleExportInputs::new(text(kernel)?, text(&workload)?, "x86_64", &archive)
        .initrd(text(&initrd.image_path)?)
        .verity(&tree, roothash.trim_end())
        .resources(1, 1024)
        .posture(PostureInputs::new(AgentProfile::Dev).allows_egress(true))
        .label("spiralcore-installed-mailbox-native-fixture")
        .local_build();
    inputs.boot_assets = Some(BootAssetsInputs {
        manifest_bytes: &local_manifest_bytes,
        manifest_sha256: &local_manifest_sha,
        runtime_overlay: &overlay_archive,
        initramfs: &initramfs_archive,
    });
    let signer = HostBundleSigner::load_at(&home.join("keys"))?;
    let exported = export_bundle_with_signer(&inputs, &signer)?;
    let trust = FsTrustStore::default_path()?;
    mvm_core::config::create_private_dir(trust.root())?;
    let key = mvm_client::bundle::BundleSigner::verifying_key(&signer);
    let untrusted_control = trust.lookup(&exported.key_id).is_none();
    if untrusted_control {
        ensure!(
            matches!(
                verify_bundle_file(&archive, &trust),
                Err(mvm_core::plan::bundle::BundleVerifyError::UnknownKey { .. })
            ),
            "fresh bundle must fail under an untrusted publisher"
        );
    }
    match mvm_core::util::atomic_io::write_private_new(
        &trust.root().join(format!("{}.pub", exported.key_id.0)),
        key.as_bytes(),
    ) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            ensure!(
                trust.lookup(&exported.key_id) == Some(key),
                "fixture trust changed"
            );
        }
        Err(error) => return Err(error.into()),
    }
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
    for (selected, source) in [
        (
            cfg.initrd_path.as_deref(),
            input.join("initramfs/initramfs.cpio.gz"),
        ),
        (
            cfg.runtime_overlay_path.as_deref(),
            input.join("overlay/overlay.ext4"),
        ),
        (
            cfg.runtime_overlay_verity_path.as_deref(),
            input.join("overlay/overlay.verity"),
        ),
    ] {
        let selected = selected.context("exact installed runtime selection missing")?;
        ensure!(
            fs::read(selected)? == fs::read(source)?,
            "installed runtime bytes differ"
        );
    }
    ensure!(
        cfg.legacy_portable_runtime
            .as_ref()
            .context("portable proof missing")?
            .pin
            == verified.plan_artifact()
    );
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
            "local_image_set": local_manifest_sha.as_str(),
            "runtime_trust": "isolated contributor bundle publisher",
            "untrusted_control": untrusted_control,
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
