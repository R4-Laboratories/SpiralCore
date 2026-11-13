//! Opt-in release-asset qualification. Only the fixture producer may acquire
//! official assets. The consumer verifies and attaches the signed inputs offline.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use anyhow::{Context, Result, ensure};
use mvm_client::{image_admin::revocations, launch::bundle_runtime::attach_if_installed_bundle};
use mvm_core::{
    arch::GuestArch,
    crypto::image_verify::{sha256_file, sha256_reader},
    plan::{
        ArtifactRole, BundleRegistry, FsTrustStore,
        bundle::{VerifiedBundleFile, key_id_from_pubkey, verify_bundle_file},
    },
    private_fs::{create_owned_private_dir, require_current_owner},
    registry_pack::local_input::read_metadata,
    security::AgentProfile,
    util::test_env::TestEnv,
    vm_backend::VmStartConfig,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    net::Ipv4Addr,
    os::{
        fd::OwnedFd,
        unix::{
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
            net::UnixStream,
        },
    },
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

const MEMBERS: [&str; 7] = [
    "environment.json",
    "install.json",
    "profile-facts.json",
    "publisher.pub",
    "revocations.json",
    "revocations.json.bundle",
    "workload.mvmpkg",
];

fn required(key: &str) -> Result<OsString> {
    std::env::var_os(key)
        .filter(|s| !s.is_empty())
        .with_context(|| format!("missing {key}"))
}

fn open_regular(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    ensure!(file.metadata()?.is_file(), "expected a regular file");
    Ok(file)
}

struct Inputs {
    executable: PathBuf,
    elf: PathBuf,
    digest: String,
    ip: Ipv4Addr,
    document: PathBuf,
    bundle: PathBuf,
}

impl Inputs {
    fn read() -> Result<Self> {
        let path = |name: &str| -> Result<PathBuf> {
            let p = PathBuf::from(required(name)?);
            ensure!(
                p.is_absolute() && fs::symlink_metadata(&p)?.is_file(),
                "{name} must name an absolute regular file"
            );
            Ok(p)
        };
        let elf = path("SC_FLOWMUX_GUEST_ELF")?;
        let original = open_regular(&elf)?;
        let len = original.metadata()?.len();
        ensure!(
            len > 0 && len <= 256 << 20,
            "guest ELF outside fixture bound"
        );
        let mut bytes = Vec::with_capacity(len as usize);
        original.take(len + 1).read_to_end(&mut bytes)?;
        let digest = std::env::var("SC_FLOWMUX_GUEST_SHA256")?;
        ensure!(
            bytes.len() as u64 == len && mvm_contract::hash::sha256_hex(&bytes) == digest,
            "qualification guest digest mismatch"
        );
        mvm_build::guest_elf::validate_static_guest_elf(&bytes, &elf, GuestArch::X86_64)?;
        let address = std::env::var("SC_FLOWMUX_FIXTURE_IPV4")?;
        let ip: Ipv4Addr = address.parse()?;
        ensure!(ip.to_string() == address, "noncanonical fixture IPv4");
        Ok(Self {
            executable: path("SC_FLOWMUX_FIXTURE_PRODUCER")?,
            elf,
            digest,
            ip,
            document: path("SC_FLOWMUX_REVOCATIONS")?,
            bundle: path("SC_FLOWMUX_REVOCATIONS_BUNDLE")?,
        })
    }

    fn run(&self, home: &Path, output: &Path) -> Result<(ExitStatus, String)> {
        let (mut reader, writer) = UnixStream::pair()?;
        let mut child = Command::new(&self.executable)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("TMPDIR", home)
            .env("HOME", home)
            .env("MVM_HOME", home)
            .arg(&self.elf)
            .arg(&self.digest)
            .arg(self.ip.to_string())
            .arg(output)
            .stdin(Stdio::null())
            .stdout(Stdio::from(OwnedFd::from(writer.try_clone()?)))
            .stderr(Stdio::from(OwnedFd::from(writer)))
            .spawn()?;
        let mut prefix = Vec::new();
        let captured = (|| -> io::Result<()> {
            (&mut reader).take(8192).read_to_end(&mut prefix)?;
            io::copy(&mut reader, &mut io::sink())?;
            Ok(())
        })();
        drop(reader);
        let status = child.wait()?;
        captured?;
        Ok((status, String::from_utf8_lossy(&prefix).into_owned()))
    }
}

// Keep phase errors public even when the canonical release client reports a
// private temporary path. Recognize the original scaffold's exact failure.
fn failure_reason(captured: &str) -> &str {
    if captured.contains("fixture producer is not implemented") {
        "fixture producer is not implemented"
    } else {
        "producer diagnostic did not match the original scaffold"
    }
}

type Inventory = BTreeMap<String, (u64, u64, u64, String)>;

fn inventory(packet: &Path) -> Result<Inventory> {
    let dir = fs::symlink_metadata(packet)?;
    ensure!(
        dir.is_dir() && dir.mode() & 0o7777 == 0o700,
        "packet directory permissions"
    );
    require_current_owner(&dir)?;
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(packet)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("non-UTF8 packet member"))?;
        ensure!(MEMBERS.contains(&name.as_str()), "unexpected packet member");
        let path = entry.path();
        let m = fs::symlink_metadata(&path)?;
        ensure!(
            m.is_file() && m.nlink() == 1 && m.mode() & 0o7777 == 0o600,
            "invalid packet member"
        );
        require_current_owner(&m)?;
        let cap = if name == "workload.mvmpkg" {
            mvm_contract::plan::bundle::MAX_BUNDLE_TOTAL_BYTES
        } else {
            1 << 20
        };
        ensure!(
            m.len() > 0 && m.len() <= cap,
            "packet member outside extent"
        );
        let file = open_regular(&path)?;
        let original = file.metadata()?;
        ensure!(
            (m.dev(), m.ino(), m.len(), m.mode(), m.nlink(), m.uid())
                == (
                    original.dev(),
                    original.ino(),
                    original.len(),
                    original.mode(),
                    original.nlink(),
                    original.uid()
                ),
            "packet member changed during reopen"
        );
        let mut bounded = file.take(m.len() + 1);
        let digest = sha256_reader(&mut bounded)?;
        ensure!(bounded.limit() == 1, "packet member length changed");
        files.insert(name, (m.dev(), m.ino(), m.len(), digest));
    }
    ensure!(files.len() == MEMBERS.len(), "packet member missing");
    Ok(files)
}

fn packet_json(packet: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&read_metadata(&packet.join(name))?)?)
}

fn verify_role(packet: &Path, verified: &VerifiedBundleFile, ip: Ipv4Addr) -> Result<()> {
    let manifest = &verified.manifest;
    let address = format!("{ip}:38443");
    let expected = json!({
        "descriptor_version": 1, "pack": {"name":"spiralcore/spiralcore-flowmux-proxy-guest","version":"0.1.0"},
        "target":"x86_64-linux","entrypoint":{"argv":["/usr/local/bin/spiralcore-flowmux-proxy-guest","denied.flowmux.test:38443",address]},
        "resources":{"cpu_cores":1,"memory_mb":256},"policy":{"allow_hosts":[address]},"capabilities":["egress"]
    });
    let env = manifest
        .environment()
        .context("verified environment missing")?;
    ensure!(
        serde_json::to_value(env)? == expected
            && packet_json(packet, "environment.json")? == expected,
        "signed role mismatch"
    );
    ensure!(
        manifest.arch == "x86_64"
            && manifest.schema_version == 7
            && manifest
                .resources
                .as_ref()
                .is_some_and(|r| r.vcpus == 1 && r.mem_mib == 256)
            && manifest
                .security_posture()
                .is_some_and(|p| p.profile == AgentProfile::SealedProd
                    && p.requires_auth
                    && p.verity_protected
                    && p.allows_egress
                    && !p.allows_volumes),
        "signed posture/resources mismatch"
    );
    let actual: mvm_build::builder_vm::GuestSidecar =
        serde_json::from_slice(&verified.read_small_verified_member(
            File::open(packet.join("workload.mvmpkg"))?,
            "mvm-meta.json",
            1 << 20,
        )?)?;
    ensure!(
        actual.sealed && !actual.accessible && actual.entrypoint_argv == env.entrypoint.argv,
        "signed guest entrypoint mismatch"
    );
    Ok(())
}

fn verify_revocation_members(
    packet: &Path,
    verified: &VerifiedBundleFile,
    inputs: &Inputs,
) -> Result<()> {
    let proof = verified
        .manifest
        .members
        .iter()
        .find_map(|member| match member {
            mvm_contract::plan::bundle::BundleMember::CanonicalLinuxProof(proof) => Some(proof),
            _ => None,
        })
        .context("canonical Linux proof missing")?;
    for (sidecar, artifact, input) in [
        (
            "revocations.json",
            &proof.revocation_document_artifact,
            &inputs.document,
        ),
        (
            "revocations.json.bundle",
            &proof.revocation_signature_bundle_artifact,
            &inputs.bundle,
        ),
    ] {
        let expected = read_metadata(input)?;
        let member = verified
            .manifest
            .find_by_name(artifact)
            .context("signed revocation member missing")?;
        ensure!(
            member.size_bytes == expected.len() as u64
                && member.sha256 == mvm_contract::hash::sha256_hex(&expected)
                && read_metadata(&packet.join(sidecar))? == expected,
            "packet revocation proof differs from independent inputs"
        );
    }
    Ok(())
}

fn verify_boot(archive: &Path, verified: &VerifiedBundleFile, trust: &FsTrustStore) -> Result<()> {
    // No VMM or host helper starts. This SDK path authenticates the actual
    // installed kernel, quartets, runtime overlay, verity tree and initramfs.
    let installed = BundleRegistry::default_path()?.install_file(archive, trust, false)?;
    ensure!(installed.manifest == verified.manifest && installed.sha256 == verified.bundle_sha256);
    let path = |role: ArtifactRole| -> Result<String> {
        installed
            .path_for_role(&role)
            .context("signed workload role missing")?
            .into_os_string()
            .into_string()
            .map_err(|_| anyhow::anyhow!("invalid artifact path"))
    };
    let mut config = VmStartConfig {
        name: "fixture-export-check".into(),
        rootfs_path: path(ArtifactRole::Rootfs)?,
        kernel_path: Some(path(ArtifactRole::Kernel)?),
        ..Default::default()
    };
    ensure!(
        attach_if_installed_bundle(&mut config, "firecracker")?,
        "canonical boot selection missing"
    );
    let kernel = config
        .canonical_kernel
        .as_ref()
        .context("original signed kernel missing")?;
    let decl = verified
        .manifest
        .find_by_role(&ArtifactRole::Kernel)
        .context("kernel declaration missing")?;
    ensure!(
        kernel.pin() == &verified.plan_artifact()
            && kernel.arch() == GuestArch::X86_64
            && kernel.kernel_sha256() == decl.sha256
            && sha256_file(&kernel.kernel_path())? == decl.sha256,
        "authenticated kernel identity mismatch"
    );
    let boot = verified
        .boot_assets
        .as_ref()
        .context("boot closure missing")?;
    let runtime = boot
        .cli_runtime
        .as_ref()
        .context("signed CLI runtime missing")?;
    let release = config
        .runtime_release_proof
        .as_ref()
        .context("verified release proof missing")?;
    ensure!(
        boot.arch == GuestArch::X86_64
            && release.arch == GuestArch::X86_64
            && runtime.version == "0.28.0-rc.2"
            && release.version == runtime.version
            && verified
                .manifest
                .find_by_name(&runtime.archive_artifact)
                .is_some_and(|a| a.sha256 == release.archive_sha256),
        "authenticated rc.2 runtime mismatch"
    );
    Ok(())
}

#[test]
#[ignore = "needs the built Linux producer, static guest ELF and current signed release inputs"]
fn signed_fixture_reopens_with_original_publisher() -> Result<()> {
    let inputs = Inputs::read()?;
    let working = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let producer_home = working.path().join("producer");
    let consumer_home = working.path().join("consumer");
    create_owned_private_dir(&producer_home)?;
    create_owned_private_dir(&consumer_home)?;
    let mut env = TestEnv::new();
    env.isolate_mvm_home(&producer_home);
    let public = mvm_hostd::audit::host_keypair::load_or_init()?.verifying;
    let key_id = key_id_from_pubkey(&public);
    let checkpoint = revocations::update(&inputs.document, &inputs.bundle)?;
    let output = working.path().join("packet");
    println!("FIXTURE_PRODUCER_START");
    let (status, captured) = inputs.run(&producer_home, &output)?;
    ensure!(
        status.success(),
        "fixture producer refused verified inputs ({status}): {}",
        failure_reason(&captured)
    );
    let original = inventory(&output)?;
    let root = fs::metadata(&output)?;
    let archive = output.join("workload.mvmpkg");
    let (_, _, size, digest) = &original["workload.mvmpkg"];
    env.isolate_mvm_home(&consumer_home);
    ensure!(
        revocations::update(&inputs.document, &inputs.bundle)? == checkpoint,
        "consumer revocation checkpoint differs from producer"
    );
    let trust = FsTrustStore::default_path()?;
    create_owned_private_dir(trust.root())?;
    fs::write(
        trust.root().join(format!("{}.pub", key_id.0)),
        public.as_bytes(),
    )?;
    let verified = verify_bundle_file(&archive, &trust)?;
    ensure!(
        verified.key_id == key_id
            && verified.bundle_sha256 == *digest
            && read_metadata(&output.join("publisher.pub"))? == public.as_bytes().as_slice(),
        "archive/publisher mismatch"
    );
    ensure!(
        packet_json(&output, "install.json")?
            == json!({"publisher_key_id":key_id.0,"expected_revocation_publication":checkpoint.publication})
    );
    ensure!(
        packet_json(&output, "profile-facts.json")?
            == json!({"allowed_ip":inputs.ip,"port":38443,"archive":{"sha256":digest,"decoded_bytes":size}})
    );
    verify_revocation_members(&output, &verified, &inputs)?;
    verify_role(&output, &verified, inputs.ip)?;
    verify_boot(&archive, &verified, &trust)?;
    ensure!(
        revocations::status()?.checkpoint() == &checkpoint,
        "consumer revocation checkpoint changed during boot verification"
    );
    let (refused, text) = inputs.run(&producer_home, &output)?;
    ensure!(
        !refused.success() && text.contains("output already exists"),
        "missing occupied-output refusal"
    );
    let after = fs::metadata(&output)?;
    ensure!(
        (root.dev(), root.ino()) == (after.dev(), after.ino()) && inventory(&output)? == original,
        "occupied packet changed"
    );
    println!(
        "FIXTURE_EXPORT_RESULT {}",
        json!({"guest_sha256":inputs.digest,"archive_sha256":digest,
        "archive_bytes":size,"publisher_key_id":key_id.0,"revocation_publication":checkpoint.publication,
        "public_members":original.len(),"canonical_runtime":"0.28.0-rc.2","repeat_refusal":true})
    );
    Ok(())
}
