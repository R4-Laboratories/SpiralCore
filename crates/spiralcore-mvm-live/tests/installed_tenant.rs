//! Real Linux daemon and signer under the installed supervisor, without a VM.
//! The qualification runner supplies the matching MVM executables and private home.
#![cfg(all(target_os = "linux", feature = "canonical-host"))]

use anyhow::{Result, ensure};
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::{InstalledParentProvider, TenantHelperLimits},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{num::NonZeroU64, os::unix::fs::PermissionsExt};

#[test]
#[ignore = "requires the installed mvm-host-agent and mvm-signer-helper from the same revision"]
fn original_ledger_funds_installed_daemon_and_signer_through_both_exits() -> Result<()> {
    let home = std::fs::canonicalize(
        std::env::var_os("MVM_HOME").ok_or_else(|| anyhow::anyhow!("private MVM_HOME required"))?,
    )?;
    ensure!(
        home.metadata()?.permissions().mode() & 0o077 == 0,
        "private home required"
    );
    // Provision the original test installation with its normal operator key.
    mvm_hostd::audit::host_keypair::load_or_init_at(&mvm_core::config::mvm_keys_dir())?;
    let config = InstalledConfig::two_endpoint_witness_profile();
    let key = InstallationKey {
        installation: [19; 32],
        profile: config.profile_hash(),
    };
    let installation = InstallationOwner::initialize(&home.join("installation"), key)?;
    let mut supervisor = InstalledSupervisor::start(installation, config)?;
    let mut provider = InstalledParentProvider::new(&supervisor)?;
    let base = supervisor.managed_resources().snapshot()?.usage;
    let limits = TenantHelperLimits {
        process_address_space_bytes: NonZeroU64::new(192 << 20)
            .expect("positive address-space limit"),
        audit_chain_bytes: NonZeroU64::new(256 << 10).expect("positive audit limit"),
    };
    let start = provider.start_tenant_helper("local", limits);
    let verdict = start.and_then(|()| -> Result<()> {
        let current = supervisor.managed_resources().snapshot()?.usage;
        ensure!(
            current.workers == base.workers + 10
                && current.host_carrier_bytes == base.host_carrier_bytes + 2 * (192 << 20)
        );
        let backend = mvm_client::LocalBackend::with_hypervisor("firecracker");
        ensure!(provider.install_sdk_registrar(&backend, "foreign").is_err());
        provider.install_sdk_registrar(&backend, "local")?;
        ensure!(provider.install_sdk_registrar(&backend, "local").is_err());
        ensure!(provider.start_tenant_helper("local", limits).is_err());
        ensure!(supervisor.managed_resources().snapshot()?.usage == current);
        drop(backend);
        Ok(())
    });
    let retirement = provider.stop_tenant_helper();
    if let Err(error) = verdict {
        return Err(error.context(format!("original retirement {retirement:?}")));
    }
    retirement?;
    ensure!(supervisor.managed_resources().snapshot()?.usage == base);
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish()?;
    Ok(())
}
