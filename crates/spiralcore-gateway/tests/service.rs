use spiralcore_gateway::service::normal_transport_end;
use spiralcore_gateway::service::{allowed_packet, Config};
#[test]
fn only_clean_authenticated_close_or_elapsed_duration_is_normal() {
    use spiralcore_gateway::transport::Error;
    let close = |code| {
        Error::Connection(quinn::ConnectionError::ApplicationClosed(
            quinn::ApplicationClose {
                error_code: quinn::VarInt::from_u32(code),
                reason: bytes::Bytes::new(),
            },
        ))
    };
    assert!(normal_transport_end(&close(0), false));
    assert!(!normal_transport_end(&close(1), false));
    assert!(!normal_transport_end(&Error::Deadline, false));
    assert!(normal_transport_end(&Error::Deadline, true));
    assert!(!normal_transport_end(&Error::Protocol, true));
}
fn config() -> serde_json::Value {
    serde_json::json!({"role":"listen","bind":"127.0.0.1:4433","peer":"127.0.0.1:4434","server_name":"gateway.test","certificate":"a.der","private_key":"a.key.der","trust_anchor":"ca.der","peer_certificate":"b.der","tun":"sc0","mode":"raw","local_addresses":["10.204.0.1"],"peer_addresses":["10.204.0.2"],"stats_path":"stats.json","ready_path":"ready.json","duration_secs":120})
}
#[test]
fn config_is_strict_and_scoped() {
    let c: Config = serde_json::from_value(config()).unwrap();
    c.validate().unwrap();
    let mut bad = config();
    bad["unknown"] = true.into();
    assert!(serde_json::from_value::<Config>(bad).is_err());
    for field in ["local_addresses", "peer_addresses"] {
        let mut bad = config();
        bad[field] = serde_json::json!([]);
        assert!(serde_json::from_value::<Config>(bad)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut bad = config();
    bad["duration_secs"] = 0.into();
    assert!(serde_json::from_value::<Config>(bad)
        .unwrap()
        .validate()
        .is_err());
}
#[test]
fn address_authorization_is_directional_and_fails_closed() {
    let c: Config = serde_json::from_value(config()).unwrap();
    let mut p = vec![0u8; 20];
    p[0] = 0x45;
    p[2..4].copy_from_slice(&20u16.to_be_bytes());
    p[12..16].copy_from_slice(&[10, 204, 0, 1]);
    p[16..20].copy_from_slice(&[10, 204, 0, 2]);
    assert!(allowed_packet(&c, &p, true));
    assert!(!allowed_packet(&c, &p, false));
    p[12] = 11;
    assert!(!allowed_packet(&c, &p, true));
    assert!(!allowed_packet(&c, &[], true));
}
