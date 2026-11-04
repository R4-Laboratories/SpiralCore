//! Parent resource policy fixtures; no canonical admission or native receipt.
#[test]
fn unqualified_host_agent_refuses_before_resource_or_helper_effects() {
    let (_home, mut supervisor, provider) = super::endpoint_tests::fixture();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(provider.prepare_owned_host_agent("local").is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

fn profile() -> super::host_agent::QualifiedHostAgentProfile {
    // Synthetic private policy limits; no actual helper is spawned by these
    // refusal fixtures and this does not qualify the real requested model.
    super::host_agent::QualifiedHostAgentProfile::fixture(super::ManagedClaim {
        host_metadata_bytes: 65536,
        workers: 2,
        ..Default::default()
    })
}

#[test]
fn foreign_supervisor_cannot_configure_original_helper() {
    let (_home, mut supervisor, provider) = super::endpoint_tests::fixture();
    let (_foreign_home, mut foreign, foreign_provider) = super::endpoint_tests::fixture();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .configure_host_agent(&foreign, "local", profile())
            .is_err()
    );
    assert!(provider.prepare_owned_host_agent("local").is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop((provider, foreign_provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
    foreign.close_admission();
    foreign.try_finish().unwrap();
}

#[test]
fn configured_helper_is_install_once_and_foreign_tenant_has_no_effects() {
    let (_home, mut supervisor, provider) = super::endpoint_tests::fixture();
    provider
        .configure_host_agent(&supervisor, "local", profile())
        .unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .configure_host_agent(&supervisor, "other", profile())
            .is_err()
    );
    assert!(provider.prepare_owned_host_agent("other").is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
#[ignore = "requires the source-pinned protected canonical host-agent/signer binaries and a private MVM_HOME"]
fn original_helper_shutdown_retains_credit_until_last_registrar_alias_drops() {
    assert!(std::env::var_os("MVM_HOST_AGENT_PATH").is_some());
    assert!(std::env::var_os("MVM_HOME").is_some());
    let keys = mvm_core::config::mvm_keys_dir();
    assert!(
        !keys.exists(),
        "functional fixture requires a fresh synthetic home"
    );
    // Same canonical enrollment as the upstream owned-host-agent fixtures.
    // This prepares test identity only; product spawn does not mint a fallback.
    let _identity = mvm_hostd::audit::host_keypair::load_or_init_at(&keys).unwrap();
    let (_home, mut supervisor, provider) = super::endpoint_tests::fixture();
    // Functional helper-custody fixture using the candidate resource terms.
    // This is not acceptance of their numeric bound, native activation or RSS.
    provider
        .configure_host_agent(
            &supervisor,
            "local",
            super::host_agent::QualifiedHostAgentProfile::fixture(super::ManagedClaim {
                host_plaintext_bytes: 40 * super::MIB,
                host_metadata_bytes: 12 * super::MIB,
                host_stack_bytes: 32 * super::MIB,
                workers: 2,
                ..Default::default()
            }),
        )
        .unwrap();
    let mut agent = provider
        .prepare_owned_host_agent("local")
        .unwrap_or_else(|error| {
            panic!(
                "synthetic helper startup failed: {}",
                super::host_agent::synthetic_start_failure_reason(&error)
            )
        });
    let registrar = agent.registrar().unwrap();
    registrar.check_current_for_tenant("local").unwrap();
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .workers,
        2
    );
    supervisor.close_admission();
    assert!(registrar.check_current_for_tenant("local").is_err());
    // Cleanup uses original retained custody after fresh admission closes.
    agent.shutdown().unwrap();
    agent.shutdown().unwrap();
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .workers,
        2
    );
    drop(agent);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .workers,
        2
    );
    drop(registrar);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .workers,
        0
    );
    drop(provider);
    supervisor.try_finish().unwrap();
}
