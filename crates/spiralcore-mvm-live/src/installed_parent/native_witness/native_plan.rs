use anyhow::{Context, Result, ensure};
use mvm_client::LaunchRequest;
use mvm_contract::{
    grants::{EgressGrant, Grants, projection::network_policy_from_grants},
    policy::network_policy::{HostPort, NetworkPolicy},
};
use mvm_core::plan::{
    AssetIdentity, AssetKind, ExecutionPlan, FsTrustStore, NetworkMode, PlanSeccompTier,
    SecretReleasePolicy, SignedExecutionPlan, SynthesisInput,
    bundle::{ArtifactRole, BundleRegistry, PlanArtifact, verify_bundle_file},
    synthesize_plan,
};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub(super) const NAME: &str = "spiralcore-installed-witness";

pub(super) struct NativeRequest {
    rootfs: PathBuf,
    rootfs_sha: String,
    pin: PlanArtifact,
    target: SocketAddr,
    name: &'static str,
    value: u8,
}

impl NativeRequest {
    pub(super) fn verified(
        asset_root: &Path,
        target: SocketAddr,
        name: &'static str,
        value: u8,
    ) -> Result<Self> {
        ensure!(asset_root.is_absolute() && asset_root.canonicalize()? == asset_root);
        ensure!(mvm_core::config::mvm_home_strict()? == asset_root.join("mvm"));
        let archive = asset_root.join("installed-native-witness.mvmpkg");
        let verified = verify_bundle_file(&archive, &FsTrustStore::default_path()?)?;
        let pin = verified.plan_artifact();
        ensure!(
            pin.bundle_sha256 == std::env::var("SPIRALCORE_NATIVE_BUNDLE_SHA256")?,
            "native fixture source bundle differs"
        );
        let registry = BundleRegistry::default_path()?;
        let installed = registry
            .find(&pin.bundle_sha256)?
            .context("native fixture not installed")?;
        ensure!(installed.manifest == verified.manifest);
        let artifact = installed
            .manifest
            .find_by_role(&ArtifactRole::Rootfs)
            .context("signed rootfs missing")?;
        let rootfs = installed.root.join(&artifact.path);
        let rootfs_sha = mvm_core::crypto::image_verify::sha256_file(&rootfs)?;
        ensure!(rootfs_sha == artifact.sha256.as_str());
        Ok(Self {
            rootfs,
            rootfs_sha,
            pin,
            target,
            name,
            value,
        })
    }

    pub(super) fn request(&self, signed: SignedExecutionPlan) -> Result<LaunchRequest> {
        Ok(LaunchRequest::builder(
            mvm_client::LifecycleMode::Transient,
            mvm_core::rootfs_source::RootfsSource::LocalPath(self.rootfs.clone()),
        )
        .name(self.name)
        .backend("firecracker")
        .signed_plan(signed)
        .command([
            "/bin/busybox".into(),
            "sh".into(),
            "-c".into(),
            include_str!("guest_roundtrip.sh").into(),
            "/bin/busybox".into(),
            self.target.ip().to_string(),
            self.target.port().to_string(),
            "1080".into(),
            self.value.to_string(),
        ])
        .build()?)
    }

    pub(super) fn plan(&self, policy: &NetworkPolicy) -> Result<ExecutionPlan> {
        let grants = Grants {
            egress: Some(EgressGrant {
                allow: vec![HostPort::new(
                    &self.target.ip().to_string(),
                    self.target.port(),
                )],
            }),
            ..Default::default()
        };
        let mapping = policy
            .payload_mapping()
            .context("native peer mapping missing")?;
        ensure!(
            network_policy_from_grants(&grants).with_payload_mapping(Some(mapping.clone()))
                == *policy
        );
        let policy_asset = AssetIdentity::new(
            AssetKind::Policy,
            "network_policy",
            mvm_core::policy::network_policy::content_sha256(policy),
        )?;
        let input = SynthesisInput::builder()
            .vm_name(self.name)
            .tenant("local")
            .backend_name("firecracker")
            .image_name("installed-native-witness")
            .image_sha256(&self.rootfs_sha)
            .seccomp_tier(PlanSeccompTier::Standard)
            .secret_release(SecretReleasePolicy::None)
            .secrets(Vec::new())
            .network_mode(NetworkMode::HostVsockProxy)
            .cpus(1)
            .mem_mib(1024)
            .disk_mib(0)
            .boot_timeout_secs(60)
            .destroy_on_exit(true)
            .grants(grants)
            .bundle_pin(self.pin.clone())
            .assets(vec![policy_asset])
            .shares(Vec::new())
            .redaction(Default::default())
            .reversible_replacement(Default::default())
            .audit_labels(Default::default())
            .services(Vec::new())
            .stream_edges(Vec::new())
            .stream_retention(Default::default())
            .build()?;
        Ok(synthesize_plan(&input)?)
    }
}
