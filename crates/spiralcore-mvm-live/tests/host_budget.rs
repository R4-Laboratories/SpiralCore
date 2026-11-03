use spiralcore_mvm_live::host_budget::*;

fn limits(links: u64, frame_slots: u64, serialized_buffer_bytes: u64) -> TransportLimits {
    TransportLimits {
        links,
        frame_slots,
        serialized_buffer_bytes,
    }
}
fn owner() -> HostTransportBudget {
    HostTransportBudget::new(TransportBudgetConfig {
        host: limits(3, 3, 24),
        tenant: limits(2, 2, 16),
        max_tenants: 2,
        max_frame_bytes: 12,
    })
    .unwrap()
}

#[test]
fn independent_links_share_host_limits_and_frames_pin_link_lifetime() {
    let owner = owner();
    let other = owner.clone();
    let first = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
    let second = other.reserve_link(&other.scope(1).unwrap()).unwrap();
    let a = first.reserve_frame(12).unwrap();
    let b = second.reserve_frame(12).unwrap();
    let before = owner.snapshot().unwrap();
    assert!(matches!(
        second.reserve_frame(1),
        Err(TransportBudgetError::Full)
    ));
    assert_eq!(owner.snapshot().unwrap().usage, before.usage);
    drop(first);
    assert_eq!(owner.snapshot().unwrap().usage, limits(2, 2, 24));
    drop(a);
    assert_eq!(owner.snapshot().unwrap().usage, limits(1, 1, 12));
    drop(second);
    drop(b);
    let report = owner.snapshot().unwrap();
    assert_eq!(report.usage, limits(0, 0, 0));
    assert_eq!(report.high_water, limits(2, 2, 24));
    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["scope"], "orchestrator");
    assert_eq!(json["resource_domain"], "serialized-ciphertext-transport");
    assert_eq!(json["version"], 1);
    assert_eq!(json.as_object().unwrap().len(), 9);
}

#[test]
fn tenant_scope_and_link_limits_refuse_without_partial_charges() {
    let owner = owner();
    let a = owner.scope(0).unwrap();
    let b = owner.scope(1).unwrap();
    let foreign = self::owner().scope(0).unwrap();
    assert!(matches!(
        owner.scope(2),
        Err(TransportBudgetError::UnknownScope)
    ));
    assert!(matches!(
        owner.reserve_link(&foreign),
        Err(TransportBudgetError::UnknownScope)
    ));
    let first = owner.reserve_link(&a).unwrap();
    let _second = owner.reserve_link(&a).unwrap();
    assert!(matches!(
        owner.reserve_link(&a),
        Err(TransportBudgetError::Full)
    ));
    let third = owner.reserve_link(&b).unwrap();
    assert!(matches!(
        owner.reserve_link(&b),
        Err(TransportBudgetError::Full)
    ));
    let _frame = first.reserve_frame(12).unwrap();
    assert!(matches!(
        first.reserve_frame(5),
        Err(TransportBudgetError::Full)
    ));
    let _other = third.reserve_frame(12).unwrap();
    assert!(matches!(
        first.reserve_frame(0),
        Err(TransportBudgetError::Full)
    ));
    assert!(matches!(
        first.reserve_frame(13),
        Err(TransportBudgetError::Full)
    ));
    assert_eq!(owner.snapshot().unwrap().usage, limits(3, 2, 24));
    assert_eq!(owner.snapshot().unwrap().registered_tenants, 2);
}

#[test]
fn arithmetic_overflow_refuses_atomically_and_released_capacity_is_reusable() {
    let owner = HostTransportBudget::new(TransportBudgetConfig {
        host: limits(2, 2, u64::MAX),
        tenant: limits(2, 2, u64::MAX),
        max_tenants: 1,
        max_frame_bytes: u64::MAX,
    })
    .unwrap();
    let link = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
    let first = link.reserve_frame(u64::MAX).unwrap();
    assert!(matches!(
        link.reserve_frame(1),
        Err(TransportBudgetError::Overflow)
    ));
    assert_eq!(owner.snapshot().unwrap().usage, limits(1, 1, u64::MAX));
    drop(first);
    assert!(link.reserve_frame(1).is_ok());
    assert_eq!(owner.snapshot().unwrap().usage, limits(1, 0, 0));
}

#[test]
fn host_and_tenant_frame_slots_limit_independently_of_bytes() {
    for (host_slots, tenant_slots) in [(1, 2), (2, 1)] {
        let owner = HostTransportBudget::new(TransportBudgetConfig {
            host: limits(2, host_slots, 24),
            tenant: limits(2, tenant_slots, 24),
            max_tenants: 1,
            max_frame_bytes: 12,
        })
        .unwrap();
        let link = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
        let _frame = link.reserve_frame(1).unwrap();
        assert!(matches!(
            link.reserve_frame(1),
            Err(TransportBudgetError::Full)
        ));
        assert_eq!(owner.snapshot().unwrap().usage, limits(1, 1, 1));
    }
}

#[test]
fn held_frames_do_not_hold_ledger_mutex_or_block_other_tenants() {
    let owner = owner();
    let first = owner.reserve_link(&owner.scope(0).unwrap()).unwrap();
    let frame = first.reserve_frame(12).unwrap();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let second = owner.reserve_link(&owner.scope(1).unwrap()).unwrap();
                let _frame = second.reserve_frame(12).unwrap();
                assert_eq!(owner.snapshot().unwrap().usage, limits(2, 2, 24));
            })
            .join()
            .unwrap();
    });
    drop(frame);
    assert_eq!(owner.snapshot().unwrap().usage, limits(1, 0, 0));
}

#[test]
fn invalid_configuration_and_fractional_report_counters_are_rejected() {
    assert!(matches!(
        HostTransportBudget::new(TransportBudgetConfig {
            host: limits(1, 1, 1),
            tenant: limits(1, 1, 1),
            max_tenants: 0,
            max_frame_bytes: 1,
        }),
        Err(TransportBudgetError::InvalidConfig)
    ));
    let mut report = serde_json::to_value(owner().snapshot().unwrap()).unwrap();
    report["usage"]["links"] = serde_json::json!(0.5);
    assert!(serde_json::from_value::<TransportBudgetSnapshot>(report).is_err());
    for (field, invalid) in [
        ("version", serde_json::json!(2)),
        ("resource_domain", serde_json::json!("plaintext")),
        ("scope", serde_json::json!("host")),
        ("unexpected", serde_json::json!(0)),
        ("high_water", serde_json::json!(limits(0, 0, u64::MAX))),
    ] {
        let mut report = serde_json::to_value(owner().snapshot().unwrap()).unwrap();
        report[field] = invalid;
        assert!(serde_json::from_value::<TransportBudgetSnapshot>(report).is_err());
    }
}
