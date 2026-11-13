use anyhow::{Context, Result, ensure};
use ed25519_dalek::{
    SigningKey, VerifyingKey,
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey},
};
use mvm_contract::peer::PeerBinding;
use mvm_contract::{
    grants::{CpuGrant, EgressGrant, Grants, WallClockGrant},
    policy::network_policy::{
        CooperatingPayloadPolicy, CooperatingTcpPeer, HostPort, PayloadMappingProfile,
    },
};
use mvm_core::{
    config,
    plan::{
        self, AssetIdentity, AssetKind, BundleRegistry, FsTrustStore, NetworkMode, PlanSeccompTier,
        SecretReleasePolicy, StreamRetention, SynthesisInput,
    },
    user_config,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

use installed_flowmux_witness::input::{
    Profile, capture_archive, funded_scratch, parse_inputs, staged,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Install {
    publisher_key_id: String,
    expected_revocation_publication: u64,
}

fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
fn require_role(
    env: &mvm_contract::plan::environment::EnvironmentDescriptor,
    address: &str,
) -> Result<()> {
    ensure!(
        env.entrypoint.argv
            == [
                "/usr/local/bin/spiralcore-flowmux-proxy-guest",
                "denied.flowmux.test:38443",
                address
            ]
            && env.target == mvm_contract::pack_spec::PackTarget::X86_64Linux
            && env
                .resources
                .as_ref()
                .is_some_and(|r| r.cpu_cores == 1 && r.memory_mb == 256)
            && env
                .policy
                .as_ref()
                .is_some_and(|p| p.allow_hosts.as_slice() == [address])
            && env.capabilities == [mvm_contract::plan::environment::EnvironmentCapability::Egress]
            && env.volumes.is_empty()
            && env.secret_slots.is_empty()
            && env.services.is_empty(),
        "signed role or destination mismatch"
    );
    Ok(())
}

pub fn run() -> Result<()> {
    let directory = parse_inputs(std::env::args_os().skip(1))?;
    let inputs = staged(&directory)?;
    let profile: Profile =
        serde_json::from_slice(&inputs.read_private_member("profile.json", 8192)?)?;
    profile.check()?;
    let install: Install =
        serde_json::from_slice(&inputs.read_private_member("install.json", 4096)?)?;
    let home = installed_flowmux_witness::input::explicit_home()?;
    let _fresh = mvm_core::transcript::secure_cleanup::CaptureDirectory::for_writer(&home)?;
    ensure!(
        fs::read_dir(&home)?.next().is_none(),
        "empty original MVM_HOME required"
    );
    let scratch = funded_scratch(&home)?;
    let host = profile.allowed_ip.to_string();
    let port = profile.port;
    let issuer = SigningKey::from_pkcs8_der(&zeroize::Zeroizing::new(
        inputs.read_private_member("issuer.der", 4096)?,
    ))?;
    let issuer_public =
        VerifyingKey::from_public_key_der(&inputs.read_private_member("issuer.spki", 4096)?)?;
    ensure!(
        issuer.verifying_key() == issuer_public,
        "plan issuer public key mismatch"
    );
    let peer_public =
        VerifyingKey::from_public_key_der(&inputs.read_private_member("peer.spki", 4096)?)?;
    let spki: [u8; 32] = Sha256::digest(peer_public.to_public_key_der()?.as_bytes()).into();
    let publisher = VerifyingKey::from_bytes(
        &inputs
            .read_private_member("publisher.pub", 32)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("publisher must be 32 bytes"))?,
    )?;
    let key_id = plan::bundle::key_id_from_pubkey(&publisher);
    ensure!(
        key_id.0 == install.publisher_key_id,
        "publisher identity mismatch"
    );
    let revocations = inputs.read_private_member("revocations.json", 1 << 20)?;
    let signature = inputs.read_private_member("revocations.json.bundle", 1 << 20)?;
    let current = mvm_core::image_set_revocation::verify_image_set_revocations(
        &revocations,
        &signature,
        chrono::Utc::now(),
        None,
    )?;
    ensure!(
        current.checkpoint().publication == install.expected_revocation_publication,
        "revocation publication mismatch"
    );
    let expected_env: mvm_contract::plan::environment::EnvironmentDescriptor =
        serde_json::from_slice(&inputs.read_private_member("environment.json", 64 << 10)?)?;
    let original = capture_archive(&directory, "workload.mvmpkg", &profile.archive, &scratch)?;
    let verifier = tempfile::tempdir_in(&scratch)?;
    let trust = FsTrustStore::new(verifier.path());
    let public_leaf = format!("{}.pub", key_id.0);
    create(&trust.root().join(&public_leaf), publisher.as_bytes())?;
    let verified = plan::bundle::verify_bundle_file(original.path(), &trust)?;
    let pin = verified.plan_artifact();
    profile.matches_archive(&pin.bundle_sha256, original.as_file().metadata()?.len())?;
    ensure!(verified.key_id == key_id, "signed publisher mismatch");
    let env = verified
        .manifest
        .environment()
        .context("signed environment missing")?;
    ensure!(
        *env == expected_env,
        "signed environment differs from operator expectation"
    );
    require_role(env, &profile.address().to_string())?;
    let app = verified
        .manifest
        .find_by_role(&plan::ArtifactRole::Rootfs)
        .context("signed root required")?;
    let kernel = verified
        .manifest
        .find_by_role(&plan::ArtifactRole::Kernel)
        .context("signed kernel required")?;
    let mapping = CooperatingPayloadPolicy::new(vec![CooperatingTcpPeer {
        host: host.clone(),
        port,
        server_name: profile.server_name.clone(),
        spki_sha256: spki,
        profile: PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
    }])?;
    let peers: Vec<PeerBinding> = Vec::new();
    let grants = Grants {
        cpu: Some(CpuGrant::Share { millicores: 1000 }),
        wall_clock: Some(WallClockGrant::Secs {
            secs: 90.try_into()?,
        }),
        egress: Some(EgressGrant {
            allow: vec![HostPort::new(host.as_str(), port)],
        }),
        ..Default::default()
    };
    let network = mvm_contract::grants::projection::network_policy_from_grants(&grants)
        .with_peers(peers.clone())
        .with_payload_mapping(Some(mapping.clone()));
    ensure!(plan::environment_network_permitted(env, &network));
    let net_digest = mvm_core::policy::network_policy::content_sha256(&network);
    let policy = AssetIdentity::new(AssetKind::Policy, "network_policy", net_digest)?;
    let verbs = vec![
        plan::VerbId::new("activate-environment")?,
        plan::VerbId::new("run-entrypoint")?,
    ];
    let input = SynthesisInput::builder()
        .agent_verbs(verbs.clone())
        .vm_name("flowmux-app")
        .tenant("local")
        .backend_name("firecracker")
        .image_name("spiralcore/spiralcore-flowmux-proxy-guest@0.1.0")
        .image_sha256(&app.sha256)
        .kernel_sha256(kernel.sha256.as_str())
        .seccomp_tier(PlanSeccompTier::Network)
        .secret_release(SecretReleasePolicy::None)
        .secrets(vec![])
        .network_mode(NetworkMode::HostVsockProxy)
        .grants(grants)
        .cpus(1)
        .mem_mib(256)
        .disk_mib(0)
        .boot_timeout_secs(60)
        .destroy_on_exit(true)
        .bundle_pin(pin.clone())
        .shares(vec![])
        .assets(vec![policy])
        .redaction(Default::default())
        .reversible_replacement(Default::default())
        .audit_labels(Default::default())
        .services(vec![])
        .stream_edges(vec![])
        .stream_retention(StreamRetention::Ephemeral)
        .build()?;
    let plan = plan::synthesize_plan(&input)?;
    let signed = plan::sign_plan(&plan, &issuer, profile.issuer_id.as_str());
    let checked = plan::verify_plan(
        &signed,
        &[(profile.issuer_id.as_str(), &issuer.verifying_key())],
    )?;
    ensure!(
        checked.agent_verbs.as_ref() == Some(&verbs),
        "verified plan omitted original entrypoint grant"
    );
    mvm_core::policy::direct_plan::AdmittedNetworkPlan::from_plan(&checked, "fm-a", &network)?;
    let current =
        mvm_core::image_set_revocation::update_image_set_revocations(&revocations, &signature)?;
    ensure!(
        current.publication == install.expected_revocation_publication,
        "installed revocation publication mismatch"
    );
    let installed_trust = FsTrustStore::default_path()?;
    config::create_private_dir(installed_trust.root())?;
    create(
        &installed_trust.root().join(public_leaf),
        publisher.as_bytes(),
    )?;
    let registry = BundleRegistry::default_path()?;
    let installed = registry.install_file(original.path(), &installed_trust, false)?;
    ensure!(
        installed.sha256 == pin.bundle_sha256 && installed.manifest == verified.manifest,
        "installed bundle changed"
    );
    let key_directory = config::mvm_keys_dir();
    config::create_private_dir(&key_directory)?;
    let host_signer = mvm_hostd::audit::host_keypair::load_or_init_at(&key_directory)?;
    ensure!(
        fs::read(key_directory.join("host-signer.pub"))?.as_slice()
            == host_signer.verifying_key().as_bytes(),
        "host signer public half mismatch"
    );
    drop(host_signer);
    let config = user_config::MvmConfig {
        trusted_plan_signers: vec![user_config::TrustedPlanSigner {
            signer_id: profile.issuer_id,
            ed25519_pubkey_hex: hex::encode(issuer_public.to_bytes()),
        }],
        egress_proxy: Some(Default::default()),
        ..Default::default()
    };
    user_config::save(&config, None)?;
    let rootfs = installed
        .path_for_role(&plan::ArtifactRole::Rootfs)
        .context("installed signed root required")?;
    create(
        &home.join("flowmux-operator.json"),
        &serde_json::to_vec(
            &serde_json::json!({"signed_plan":signed,"bundle_pin":pin,"archive":registry.archive_path(&installed.sha256),"decoded_archive_bytes":profile.archive.decoded_bytes,"rootfs":rootfs,"mapping":mapping,"peers":peers,"peer_spki_sha256":hex::encode(spki),"vm_name":"fm-a","workload":"flowmux-app","tenant":"local"}),
        )?,
    )?;
    fs::File::open(&home)?.sync_all()?;
    println!("installed verified package and original signed plan");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mvm_contract::plan::environment::EnvironmentDescriptor;
    #[test]
    fn signed_role_must_match_exact_literal_binary_and_resources() -> Result<()> {
        let address = "192.0.2.7:38443";
        let mut env: EnvironmentDescriptor = serde_json::from_value(serde_json::json!({
            "descriptor_version":1,"target":"x86_64-linux",
            "entrypoint":{"argv":["/usr/local/bin/spiralcore-flowmux-proxy-guest","denied.flowmux.test:38443",address]},
            "resources":{"cpu_cores":1,"memory_mb":256},"policy":{"allow_hosts":[address]},"capabilities":["egress"]
        }))?;
        require_role(&env, address)?;
        assert!(require_role(&env, "192.0.2.8:38443").is_err());
        let original = env.clone();
        env.entrypoint.argv[0] = "/some/other/binary".into();
        assert!(require_role(&env, address).is_err());
        env = original.clone();
        env.policy
            .as_mut()
            .context("missing policy")?
            .allow_hosts
            .push("extra.example:443".into());
        assert!(require_role(&env, address).is_err());
        env = original;
        env.resources
            .as_mut()
            .context("missing resources")?
            .memory_mb = 1024;
        assert!(require_role(&env, address).is_err());
        Ok(())
    }
}
