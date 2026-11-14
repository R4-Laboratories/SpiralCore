use anyhow::{Context, Result, ensure};
use ed25519_dalek::VerifyingKey;
use mvm_core::{
    crypto::image_verify::sha256_file,
    image_set_revocation::ImageSetRevocationStore,
    plan::{
        ArtifactRole, FsTrustStore, PlanArtifact,
        bundle::{BundleInstallError, BundleRegistry, key_id_from_pubkey, verify_bundle_file},
    },
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::SystemTime,
};

fn home() -> Result<PathBuf> {
    let declared =
        PathBuf::from(std::env::var_os("MVM_HOME").context("explicit private MVM_HOME required")?);
    ensure!(declared.is_absolute(), "MVM_HOME must be absolute");
    ensure!(
        mvm_core::config::mvm_home_strict()? == declared,
        "MVM_HOME differs"
    );
    private_dir(&declared)?;
    Ok(declared)
}

fn private_dir(path: &Path) -> Result<()> {
    ensure!(
        fs::canonicalize(path)?.as_os_str() == path.as_os_str(),
        "expected unaliased private directory"
    );
    let metadata = fs::symlink_metadata(path)?;
    mvm_core::private_fs::require_current_owner(&metadata)?;
    ensure!(
        metadata.is_dir() && metadata.mode() & 0o7777 == 0o700,
        "expected private directory"
    );
    Ok(())
}

fn read_private(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    mvm_core::private_fs::require_current_owner(&metadata)?;
    ensure!(
        metadata.is_file()
            && metadata.nlink() == 1
            && metadata.mode() & 0o077 == 0
            && metadata.len() <= cap,
        "expected bounded private regular input"
    );
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 == metadata.len(), "input length changed");
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn publisher(hexadecimal: &str) -> Result<VerifyingKey> {
    ensure!(
        hexadecimal.len() == 64
            && hexadecimal
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "expected lowercase 32-byte publisher public key"
    );
    let bytes: [u8; 32] = hex::decode(hexadecimal)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("publisher key length differs"))?;
    Ok(VerifyingKey::from_bytes(&bytes)?)
}

fn verify_publisher(path: &Path, expected: &VerifyingKey) -> Result<()> {
    ensure!(
        read_private(path, 32)? == expected.as_bytes(),
        "packet publisher differs from external pin"
    );
    Ok(())
}

fn publish_equal(path: &Path, bytes: &[u8], cap: u64) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => ensure!(
            read_private(path, cap)? == bytes,
            "existing pinned input differs"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => write_new(path, bytes)?,
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub(super) fn initialize() -> Result<()> {
    let home = home()?;
    let keys = mvm_hostd::audit::host_keypair::default_keys_dir()?;
    ensure!(keys == home.join("keys"), "host signer path differs");
    let signer = mvm_hostd::audit::host_keypair::load_or_init_at(&keys)?;
    println!(
        "{}",
        serde_json::json!({"publisher_public_key_hex": hex::encode(signer.verifying.as_bytes()), "publisher_key_id": key_id_from_pubkey(&signer.verifying).0})
    );
    Ok(())
}

struct RoleInput {
    packet: PathBuf,
    pin: PlanArtifact,
    bytes: u64,
    verified_manifest: mvm_core::plan::bundle::BundleManifest,
}

fn authenticated_installed_root(
    installed: &mvm_core::plan::bundle::InstalledBundle,
    authenticated: &mvm_core::plan::bundle::BundleManifest,
) -> Result<(PathBuf, String)> {
    ensure!(
        installed.manifest == *authenticated,
        "cached manifest differs from authenticated archive"
    );
    let member = authenticated
        .find_by_role(&ArtifactRole::Rootfs)
        .context("authenticated workload root missing")?;
    let root = installed.root.join(&member.path);
    let metadata = fs::symlink_metadata(&root)?;
    ensure!(
        metadata.is_file()
            && metadata.len() == member.size_bytes
            && sha256_file(&root)? == member.sha256,
        "installed workload root differs from authenticated archive"
    );
    Ok((root, member.sha256.clone()))
}

pub(super) fn install(packet: &Path, external_key: &str) -> Result<()> {
    let expected = publisher(external_key)?;
    let home = home()?;
    private_dir(packet)?;
    let operator = read_private(&packet.join("collector-operator.json"), 256 << 10)?;
    let parsed: serde_json::Value = serde_json::from_slice(&operator)?;
    let object = parsed.as_object().context("operator must be an object")?;
    ensure!(object.len() == 1, "unknown operator fields");
    let roles = object
        .get("roles")
        .and_then(|v| v.as_array())
        .context("operator roles missing")?;
    ensure!(roles.len() == 2, "operator needs exactly two roles");
    let scratch = tempfile::Builder::new()
        .prefix("collector-pinned-trust-")
        .tempdir_in(&home)?;
    let trust = FsTrustStore::new(scratch.path());
    let key_id = key_id_from_pubkey(&expected);
    write_new(
        &trust.root().join(format!("{}.pub", key_id.0)),
        expected.as_bytes(),
    )?;
    let current =
        ImageSetRevocationStore::in_mvm_home().capture_applied_proof(SystemTime::now().into())?;
    let mut inputs = Vec::with_capacity(2);
    for (index, (name, guest)) in [
        ("sc-journal-a", "/usr/local/bin/spiralcore-durable-source"),
        (
            "sc-journal-b",
            "/usr/local/bin/spiralcore-durable-destination",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let role = roles[index].as_object().context("role must be an object")?;
        ensure!(role.len() == 2, "unknown role fields");
        let pin: PlanArtifact =
            serde_json::from_value(role.get("bundle_pin").context("role pin missing")?.clone())?;
        let bytes = role
            .get("decoded_archive_bytes")
            .and_then(|v| v.as_u64())
            .context("role archive length missing")?;
        ensure!(
            bytes > 0 && bytes <= 128 << 20 && pin.key_id == key_id,
            "role archive or publisher differs"
        );
        let role_packet = packet.join(name);
        private_dir(&role_packet)?;
        verify_publisher(&role_packet.join("publisher.pub"), &expected)?;
        ensure!(
            read_private(&role_packet.join("revocations.json"), 256 << 10)? == current.document()
                && read_private(&role_packet.join("revocations.json.bundle"), 256 << 10)?
                    == current.signature_bundle(),
            "packet revocations differ from applied canonical proof"
        );
        let archive = role_packet.join("workload.mvmpkg");
        let meta = fs::symlink_metadata(&archive)?;
        mvm_core::private_fs::require_current_owner(&meta)?;
        ensure!(
            meta.is_file() && meta.nlink() == 1 && meta.mode() & 0o077 == 0 && meta.len() == bytes,
            "archive extent or ownership differs"
        );
        let verified = verify_bundle_file(&archive, &trust)?;
        ensure!(verified.plan_artifact() == pin, "verified role pin differs");
        let mut proofs = verified
            .manifest
            .members
            .iter()
            .filter_map(|member| match member {
                mvm_contract::plan::bundle::BundleMember::CanonicalLinuxProof(proof) => Some(proof),
                _ => None,
            });
        let proof = proofs.next().context("canonical Linux proof missing")?;
        ensure!(proofs.next().is_none(), "ambiguous canonical Linux proof");
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
                "embedded canonical revocations differ"
            );
        }
        let env = verified
            .manifest
            .environment()
            .context("signed role environment missing")?;
        ensure!(
            env.entrypoint.argv == [guest]
                && env.entrypoint.working_dir.is_none()
                && env
                    .pack
                    .as_ref()
                    .is_some_and(|pack| pack.name == format!("spiralcore/{name}"))
                && env
                    .resources
                    .as_ref()
                    .is_some_and(|resources| resources.cpu_cores == 1 && resources.memory_mb == 256)
                && env.policy.as_ref().is_some_and(
                    |policy| policy.allow_hosts.is_empty() && policy.request_budget.is_none()
                )
                && env.capabilities.is_empty()
                && env.services.is_empty()
                && env.secret_slots.is_empty() && env.volumes.is_empty(),
            "signed role environment differs"
        );
        ensure!(
            verified
                .manifest
                .security_posture()
                .is_some_and(|posture| posture.profile
                    == mvm_core::security::AgentProfile::SealedProd
                    && posture.requires_auth
                    && posture.verity_protected
                    && !posture.allows_egress
                    && !posture.allows_volumes),
            "signed role posture differs"
        );
        ensure!(
            verified
                .boot_assets
                .as_ref()
                .is_some_and(|boot| boot.arch == mvm_core::arch::GuestArch::X86_64
                    && boot.cli_runtime.is_some()),
            "canonical role boot assets missing"
        );
        inputs.push(RoleInput {
            packet: role_packet,
            pin,
            bytes,
            verified_manifest: verified.manifest,
        });
    }
    ensure!(
        inputs[0].pin.bundle_sha256 != inputs[1].pin.bundle_sha256,
        "role bundles must differ"
    );
    let persistent_trust = FsTrustStore::default_path()?;
    mvm_core::config::create_private_dir(persistent_trust.root())?;
    let key_file = persistent_trust.root().join(format!("{}.pub", key_id.0));
    publish_equal(&key_file, expected.as_bytes(), 32)?;
    File::open(persistent_trust.root())?.sync_all()?;
    let registry = BundleRegistry::default_path()?;
    let mut installed = Vec::with_capacity(2);
    for input in inputs {
        let bundle = match registry.install_file(
            &input.packet.join("workload.mvmpkg"),
            &persistent_trust,
            false,
        ) {
            Ok(bundle) => bundle,
            Err(BundleInstallError::AlreadyInstalled { bundle_sha256 })
                if bundle_sha256 == input.pin.bundle_sha256 =>
            {
                registry
                    .find(&bundle_sha256)?
                    .context("verified existing bundle missing")?
            }
            Err(error) => return Err(error.into()),
        };
        ensure!(
            bundle.sha256 == input.pin.bundle_sha256,
            "installed address differs"
        );
        let (rootfs, rootfs_sha256) =
            authenticated_installed_root(&bundle, &input.verified_manifest)?;
        installed.push(serde_json::json!({"bundle_pin": input.pin, "decoded_archive_bytes": input.bytes, "installed_root": bundle.root, "rootfs": rootfs, "rootfs_sha256": rootfs_sha256}));
    }
    let final_proof =
        ImageSetRevocationStore::in_mvm_home().capture_applied_proof(SystemTime::now().into())?;
    ensure!(
        final_proof.document() == current.document()
            && final_proof.signature_bundle() == current.signature_bundle(),
        "canonical applied proof changed during installation"
    );
    publish_equal(&home.join("collector-operator.json"), &operator, 256 << 10)?;
    File::open(&home)?.sync_all()?;
    println!(
        "{}",
        serde_json::json!({"roles": installed, "operator": home.join("collector-operator.json")})
    );
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
