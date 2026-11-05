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
    let home = mvm_core::config::mvm_home_strict().unwrap();
    let operator = mvm_core::config::mvm_config_dir_at(&home);
    std::fs::create_dir(&operator).unwrap();
    std::fs::write(operator.join("config.toml"), "[egress_proxy]\n").unwrap();
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
            super::host_agent::QualifiedHostAgentProfile::fixture_with_cold_operator(
                super::ManagedClaim {
                    host_plaintext_bytes: 40 * super::MIB,
                    host_metadata_bytes: 12 * super::MIB,
                    host_stack_bytes: 32 * super::MIB,
                    workers: 2,
                    ..Default::default()
                },
                operator,
                Some("on".into()),
            ),
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

#[test]
fn cold_operator_settings_require_explicit_direct_and_keep_original_credit() {
    use super::{MIB, ManagedClaim, tenant_tag};
    use std::fs;

    let (home, mut supervisor, provider) = super::endpoint_tests::fixture();
    let config_dir = home.path().join("operator");
    fs::create_dir(&config_dir).unwrap();
    let config = config_dir.join("config.toml");
    let pool = supervisor.managed_resources();
    let initial = pool.snapshot().unwrap();
    for body in [
        "",
        "[egress_proxy",
        "[egress_proxy]\nhttp = 'http://proxy.invalid:3128'",
    ] {
        fs::write(&config, body).unwrap();
        assert!(
            provider
                .capture_cold_operator_settings("local", &config_dir, Some("on"))
                .is_err()
        );
        assert_eq!(pool.snapshot().unwrap().usage, initial.usage);
    }
    fs::write(&config, "[egress_proxy]\n").unwrap();
    for selection in [None, Some(""), Some("ture"), Some(" OFF ")] {
        assert!(
            provider
                .capture_cold_operator_settings("local", &config_dir, selection)
                .is_err(),
            "installed cold profile accepted a missing or non-affirmative telemetry selection"
        );
        assert_eq!(pool.snapshot().unwrap().usage, initial.usage);
    }
    let saturated = pool
        .reserve(
            tenant_tag("local"),
            ManagedClaim {
                host_metadata_bytes: 256 * MIB,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        provider
            .capture_cold_operator_settings("local", &config_dir, Some("on"))
            .is_err()
    );
    drop(saturated);
    let captured = provider
        .capture_cold_operator_settings("local", &config_dir, Some("on"))
        .unwrap();
    fs::write(
        &config,
        "[egress_proxy]\nhttp = 'http://changed.invalid:3128'",
    )
    .unwrap();
    let mvm_vmm::host::network_endpoint_spawn::InstalledEndpointSettings::ResolvedConfigured(proxy) =
        captured.as_ref()
    else {
        panic!("explicit operator table must remain an explicit override");
    };
    assert_eq!(proxy, &Default::default());
    let held = pool.snapshot().unwrap();
    assert!(held.usage.host_metadata_bytes > initial.usage.host_metadata_bytes);
    assert_eq!(held.active_reservations, initial.active_reservations + 1);
    drop(captured);
    assert_eq!(pool.snapshot().unwrap().usage, initial.usage);
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn invalid_cold_operator_settings_refuse_before_original_helper_spawn() {
    let (home, mut supervisor, provider) = super::endpoint_tests::fixture();
    let config_dir = home.path().join("operator");
    std::fs::create_dir(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "[egress_proxy\n").unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap();
    provider
        .configure_host_agent(
            &supervisor,
            "local",
            super::host_agent::QualifiedHostAgentProfile::fixture_with_cold_operator(
                super::ManagedClaim {
                    host_metadata_bytes: 65536,
                    workers: 2,
                    ..Default::default()
                },
                config_dir,
                Some("on".into()),
            ),
        )
        .unwrap();
    let error = provider.prepare_owned_host_agent("local").err().unwrap();
    assert!(error.to_string().contains("host config is invalid"));
    let after = supervisor.managed_resources().snapshot().unwrap();
    assert_eq!(after.usage, before.usage);
    assert_eq!(after.active_reservations, before.active_reservations);
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}
