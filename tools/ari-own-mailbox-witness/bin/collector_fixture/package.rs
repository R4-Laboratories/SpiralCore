use super::{
    Role,
    stage::{private_dir, write_new},
};
use anyhow::{Context, Result, ensure};
use mvm_build::{
    builder_vm::GuestSidecar,
    oci_runtime_inject::{ImageRuntimeConfig, inject_mvm_runtime},
    rootfs::{MaterializeExt4Input, materialize_ext4_rejecting_unsupported},
    run_image::unpacked_tree_size,
};
use mvm_client::{
    bundle::{BundleExportInputs, BundleSigner, HostBundleSigner, PostureInputs},
    portable_boot_assets::PreparedPortableBootAssets,
};
use mvm_contract::{
    pack_spec::{PackIdentity, PackResources, PackTarget},
    plan::environment::{
        ENVIRONMENT_DESCRIPTOR_VERSION, EnvironmentDescriptor, EnvironmentEntrypoint,
        EnvironmentPolicy,
    },
};
use mvm_core::{
    arch::GuestArch,
    crypto::image_verify::sha256_file,
    image_set_revocation::{ImageSetRevocationStore, RetainedImageSetRevocationProof},
    plan::{
        ArtifactRole, FsTrustStore,
        bundle::{key_id_from_pubkey, verify_bundle_file},
    },
    registry_pack::local_input::read_metadata,
    security::AgentProfile,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::SystemTime,
};

const VERSION: &str = "0.1.0";
pub(super) const MEMORY_MIB: u32 = 256;

pub(super) fn environment(role: Role) -> Result<EnvironmentDescriptor> {
    let descriptor = EnvironmentDescriptor {
        descriptor_version: ENVIRONMENT_DESCRIPTOR_VERSION,
        pack: Some(PackIdentity {
            name: role.pack().into(),
            version: VERSION.into(),
        }),
        target: PackTarget::X86_64Linux,
        entrypoint: EnvironmentEntrypoint {
            argv: vec![role.guest().into()],
            working_dir: None,
        },
        resources: Some(PackResources {
            cpu_cores: 1,
            memory_mb: MEMORY_MIB,
        }),
        policy: Some(EnvironmentPolicy {
            allow_hosts: vec![],
            request_budget: None,
        }),
        capabilities: vec![],
        volumes: vec![],
        secret_slots: vec![],
        services: vec![],
    };
    descriptor.validate()?;
    Ok(descriptor)
}

fn signer() -> Result<HostBundleSigner> {
    use mvm_hostd::audit::host_keypair::{self, HostSigner};
    let home =
        PathBuf::from(std::env::var("MVM_HOME").context("explicit private MVM_HOME required")?);
    private_dir(&home)?;
    let keys = host_keypair::default_keys_dir()?;
    ensure!(keys == home.join("keys"), "signer home mismatch");
    private_dir(&keys)?;
    let secret_path = keys.join(host_keypair::SECRET_FILENAME);
    let public_path = keys.join(host_keypair::PUBLIC_FILENAME);
    for path in [&secret_path, &public_path] {
        let m = fs::symlink_metadata(path)?;
        ensure!(
            m.is_file() && m.nlink() == 1,
            "expected single-link regular key file"
        );
        mvm_core::private_fs::require_current_owner(&m)?;
    }
    let (signing, verifying) =
        mvm_core::crypto::ed25519_keypair::load_existing(&secret_path, &public_path)?;
    Ok(HostSigner {
        signing,
        verifying,
        secret_path,
        public_path,
    }
    .into())
}

fn current_revocations() -> Result<RetainedImageSetRevocationProof> {
    Ok(ImageSetRevocationStore::in_mvm_home().capture_applied_proof(SystemTime::now().into())?)
}

pub(super) fn verify_sidecar(path: &Path, argv: &[String], role: Role) -> Result<()> {
    let sidecar: GuestSidecar = serde_json::from_slice(&read_metadata(path)?)?;
    ensure!(sidecar.entrypoint_argv == argv, "baked entrypoint differs");
    ensure!(sidecar.name == role.machine(), "baked role name differs");
    ensure!(
        sidecar.sealed && !sidecar.accessible && sidecar.overlay_aware && sidecar.runtime_lean,
        "expected sealed runtime-overlay image"
    );
    Ok(())
}

pub(super) fn runtime(role: Role) -> mvm_agentd::runtime_config::RuntimeConfig {
    use mvm_agentd::runtime_config::{
        ConcurrencyConfig, InProcessMode, RuntimeConfig, WarmProcessConfig,
    };
    RuntimeConfig {
        language: "rust".into(),
        module: role.module().into(),
        function: "main".into(),
        format: "binary".into(),
        source_path: role.guest().into(),
        concurrency: Some(ConcurrencyConfig::WarmProcess(WarmProcessConfig {
            max_calls_per_worker: 2,
            max_rss_mb: 64,
            pool_size: 1,
            in_process: InProcessMode::Serial,
            max_queue_depth: Some(1),
        })),
    }
}

fn materialize(
    tree: &Path,
    work: &Path,
    env: &EnvironmentDescriptor,
    boot: &PreparedPortableBootAssets,
    role: Role,
) -> Result<PathBuf> {
    let image = work.join("image");
    fs::create_dir(&image)?;
    let rootfs = image.join("rootfs.ext4");
    let config = ImageRuntimeConfig {
        argv: env.entrypoint.argv.clone(),
        env: vec![],
        working_dir: env.entrypoint.working_dir.clone(),
    };
    inject_mvm_runtime(tree, boot.guest_binaries(), Some(&config), true)?;
    let worker = runtime(role);
    let policy = tree.join("etc/mvm/runtime.json");
    write_new(&policy, &serde_json::to_vec(&worker)?)?;
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o444))?;
    ensure!(
        mvm_agentd::runtime_config::load_from(&policy)?.as_ref() == Some(&worker),
        "baked role runtime policy differs"
    );
    let spec =
        MaterializeExt4Input::new(tree.to_owned(), rootfs.clone(), unpacked_tree_size(tree)?)
            .with_verity();
    materialize_ext4_rejecting_unsupported(&spec)?;
    GuestSidecar::for_oci_run(role.machine(), true, true)
        .with_entrypoint_argv(config.argv.clone())
        .with_libc(mvm_build::guest_libc::detect_guest_libc(tree))
        .write_to_dir(&image)?;
    let baked: ImageRuntimeConfig = serde_json::from_slice(&read_metadata(
        &tree.join(mvm_agentd::workload_env::CONFIG_REL_PATH),
    )?)?;
    ensure!(baked == config, "materialized OCI entrypoint differs");
    let wrapper = "/usr/lib/mvm/wrappers/oci-entrypoint";
    ensure!(
        fs::read_to_string(tree.join("etc/mvm/entrypoint"))?.trim_end() == wrapper
            && sha256_file(&tree.join(&wrapper[1..]))?
                == sha256_file(&boot.guest_binaries().entrypoint_runner)?,
        "materialized entrypoint wrapper differs"
    );
    verify_sidecar(&image.join("mvm-meta.json"), &config.argv, role)?;
    Ok(image)
}

struct SealedArchive {
    path: PathBuf,
    digest: String,
    bytes: u64,
}

fn export(
    image: &Path,
    packet: &Path,
    env: &EnvironmentDescriptor,
    signer: &HostBundleSigner,
    boot: &PreparedPortableBootAssets,
    role: Role,
) -> Result<SealedArchive> {
    fn utf8(p: &Path) -> Result<&str> {
        p.to_str().context("bundle path must be UTF-8")
    }
    let path = packet.join("workload.mvmpkg");
    let rootfs = image.join("rootfs.ext4");
    let verity = fs::read(image.join("rootfs.verity"))?;
    let roothash = fs::read_to_string(image.join("rootfs.roothash"))?;
    let inputs =
        BundleExportInputs::new(utf8(boot.kernel_path())?, utf8(&rootfs)?, "x86_64", &path)
            .initrd(utf8(boot.initramfs_path())?)
            .verity(&verity, roothash.trim())
            .resources(1, MEMORY_MIB)
            .label(format!("{}@{VERSION}", role.pack()))
            .posture(PostureInputs::new(AgentProfile::SealedProd).allows_egress(false))
            .environment(env.clone());
    let mut staged = boot.stage(&inputs, signer)?;
    let digest = staged.sha256()?;
    let exported = staged.publish()?;
    ensure!(
        exported.path == path && exported.key_id == key_id_from_pubkey(&signer.verifying_key()),
        "export identity differs"
    );
    let archive = fs::File::open(&path)?;
    archive.set_permissions(fs::Permissions::from_mode(0o600))?;
    archive.sync_all()?;
    Ok(SealedArchive {
        path,
        digest,
        bytes: exported.size_bytes,
    })
}

fn reopen(
    work: &Path,
    archive: &SealedArchive,
    signer: &HostBundleSigner,
) -> Result<mvm_core::plan::bundle::VerifiedBundleFile> {
    let trust_dir = super::stage::scratch(work)?;
    let trust = FsTrustStore::new(trust_dir.path());
    let public = signer.verifying_key();
    let id = key_id_from_pubkey(&public);
    write_new(
        &trust.root().join(format!("{}.pub", id.0)),
        public.as_bytes(),
    )?;
    let reopened = verify_bundle_file(&archive.path, &trust)?;
    ensure!(
        reopened.key_id == id
            && reopened.bundle_sha256 == archive.digest
            && fs::metadata(&archive.path)?.len() == archive.bytes,
        "reopened archive identity differs"
    );
    Ok(reopened)
}

fn verify_role(
    manifest: &mvm_core::plan::bundle::BundleManifest,
    image: &Path,
    env: &EnvironmentDescriptor,
    boot: &PreparedPortableBootAssets,
) -> Result<()> {
    ensure!(
        manifest.environment() == Some(env)
            && manifest.arch == "x86_64"
            && manifest
                .resources
                .as_ref()
                .is_some_and(|r| r.vcpus == 1 && r.mem_mib == MEMORY_MIB),
        "signed workload environment or size differs"
    );
    ensure!(
        manifest
            .security_posture()
            .is_some_and(|p| p.profile == AgentProfile::SealedProd
                && p.requires_auth
                && p.verity_protected
                && !p.allows_egress
                && !p.allows_volumes),
        "signed posture differs"
    );
    let sidecar = image.join("mvm-meta.json");
    let member = manifest
        .find_by_name("mvm-meta.json")
        .context("signed guest sidecar missing")?;
    ensure!(
        member.sha256 == sha256_file(&sidecar)?
            && member.size_bytes == fs::metadata(sidecar)?.len(),
        "signed guest sidecar differs"
    );
    let roothash = fs::read_to_string(image.join("rootfs.roothash"))?;
    ensure!(
        manifest
            .verity
            .as_ref()
            .is_some_and(|v| v.roothash == roothash.trim()),
        "signed root hash differs"
    );
    for (role, path) in [
        (ArtifactRole::Rootfs, image.join("rootfs.ext4")),
        (ArtifactRole::VerityHashSidecar, image.join("rootfs.verity")),
        (ArtifactRole::Kernel, boot.kernel_path().to_owned()),
        (ArtifactRole::Initrd, boot.initramfs_path().to_owned()),
    ] {
        let member = manifest
            .find_by_role(&role)
            .context("signed image member missing")?;
        ensure!(
            member.sha256 == sha256_file(&path)? && member.size_bytes == fs::metadata(path)?.len(),
            "signed image member differs"
        );
    }
    Ok(())
}

fn write_facts(
    packet: &Path,
    env: &EnvironmentDescriptor,
    role: Role,
    signer: &HostBundleSigner,
    archive: &SealedArchive,
    revocations: &RetainedImageSetRevocationProof,
) -> Result<()> {
    let public = signer.verifying_key();
    let install = serde_json::json!({
        "publisher_key_id": key_id_from_pubkey(&public).0,
        "expected_revocation_publication": revocations.verified().checkpoint().publication,
    });
    let facts = serde_json::json!({"machine": role.machine(), "guest_executable": role.guest(),
        "archive": {"sha256": archive.digest, "decoded_bytes": archive.bytes}});
    for (name, bytes) in [
        ("install.json", serde_json::to_vec_pretty(&install)?),
        ("profile-facts.json", serde_json::to_vec_pretty(&facts)?),
        ("environment.json", serde_json::to_vec_pretty(env)?),
    ] {
        write_new(&packet.join(name), &bytes)?;
    }
    write_new(&packet.join("publisher.pub"), public.as_bytes())?;
    write_new(&packet.join("revocations.json"), revocations.document())?;
    write_new(
        &packet.join("revocations.json.bundle"),
        revocations.signature_bundle(),
    )
}

pub(super) fn build(
    tree: &Path,
    role: Role,
    work: &Path,
    packet: &Path,
) -> Result<serde_json::Value> {
    let signer = signer()?;
    let original = current_revocations()?;
    let boot = PreparedPortableBootAssets::acquire(GuestArch::X86_64)?;
    let env = environment(role)?;
    let image = materialize(tree, work, &env, &boot, role)?;
    let archive = export(&image, packet, &env, &signer, &boot, role)?;
    let verified = reopen(work, &archive, &signer)?;
    verify_role(&verified.manifest, &image, &env, &boot)?;
    let closure = verified
        .boot_assets
        .as_ref()
        .context("signed boot closure missing")?;
    ensure!(
        closure.arch == GuestArch::X86_64
            && closure.cli_runtime.is_some()
            && &closure.image_set.manifest_sha256 == boot.image_set_manifest_sha256(),
        "signed boot identity differs"
    );
    boot.verify_integrity()?;
    let current = current_revocations()?;
    ensure!(
        current.document() == original.document()
            && current.signature_bundle() == original.signature_bundle(),
        "applied revocation proof changed during export"
    );
    let proof = verified
        .manifest
        .members
        .iter()
        .find_map(|m| match m {
            mvm_contract::plan::bundle::BundleMember::CanonicalLinuxProof(proof) => Some(proof),
            _ => None,
        })
        .context("canonical Linux proof missing")?;
    for (name, bytes) in [
        (&proof.revocation_document_artifact, current.document()),
        (
            &proof.revocation_signature_bundle_artifact,
            current.signature_bundle(),
        ),
    ] {
        let member = verified
            .manifest
            .find_by_name(name)
            .context("signed revocation member missing")?;
        ensure!(
            member.size_bytes == bytes.len() as u64
                && member.sha256 == mvm_contract::hash::sha256_hex(bytes),
            "archive's revocation proof differs from applied proof"
        );
    }
    ensure!(
        archive.bytes > 0 && archive.bytes <= 128 << 20,
        "role archive exceeds fixture bound"
    );
    write_facts(packet, &env, role, &signer, &archive, &current)?;
    let pin = verified.plan_artifact();
    write_new(
        &packet.join("bundle-pin.json"),
        &serde_json::to_vec_pretty(&pin)?,
    )?;
    Ok(serde_json::json!({"bundle_pin": pin, "decoded_archive_bytes": archive.bytes}))
}
