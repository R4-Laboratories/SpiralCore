use spiralcore_gateway::object_service::{run, Config};
use spiralcore_transfer::{
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
};

fn config() -> serde_json::Value {
    serde_json::json!({
        "role":"connect","bind":"127.0.0.1:0","peer":"127.0.0.1:4433",
        "server_name":"gateway.test","certificate":"local.der","private_key":"key.der",
        "trust_anchor":"ca.der","peer_certificate":"peer.der","scope":"combined-test",
        "policy_epoch":1,"store_dir":"store","stats_path":"stats.json","ready_path":"ready.json",
        "duration_secs":30,"verify_files":[],
        "groups":[{"label":"combined","mode":"combined","schema_path":"schema",
            "checkpoint_path":"checkpoint","recipe_path":"recipe","operations":[
                {"label":"checkpoint","action":"checkpoint","warm_probe":false},
                {"label":"schema","action":"schema","warm_probe":false},
                {"label":"target","action":"send","path":"target","warm_probe":true}]}]
    })
}

#[test]
fn combined_configuration_requires_both_auxiliary_artifacts_and_rejects_generic_seed() {
    let valid = config();
    serde_json::from_value::<Config>(valid.clone())
        .unwrap()
        .validate()
        .unwrap();
    for field in ["schema_path", "checkpoint_path", "recipe_path"] {
        let mut missing = valid.clone();
        missing["groups"][0][field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Config>(missing)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut bad = valid;
    bad["groups"][0]["operations"][2]["action"] = serde_json::json!("seed");
    assert!(serde_json::from_value::<Config>(bad)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn cpu_conscious_configuration_is_explicit_and_requires_combined_artifacts() {
    let mut valid = config();
    valid["groups"][0]["mode"] = serde_json::json!("combined-cpu");
    serde_json::from_value::<Config>(valid.clone())
        .unwrap()
        .validate()
        .unwrap();
    for field in ["schema_path", "checkpoint_path", "recipe_path"] {
        let mut missing = valid.clone();
        missing["groups"][0][field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Config>(missing)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut bad = valid.clone();
    bad["groups"][0]["operations"][2]["action"] = serde_json::json!("seed");
    assert!(serde_json::from_value::<Config>(bad)
        .unwrap()
        .validate()
        .is_err());
    valid["groups"][0]["mode"] = serde_json::json!("combined-cpu-unknown");
    assert!(serde_json::from_value::<Config>(valid)
        .unwrap()
        .validate()
        .is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn combined_gateway_changes_codec_within_one_authenticated_session_and_reopens_exact() {
    combined_gateway_session("combined").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cpu_conscious_gateway_skips_unfitted_schema_and_keeps_runtime_warm_and_exact_repeats() {
    combined_gateway_session("combined-cpu").await;
}

async fn combined_gateway_session(mode: &str) {
    use rcgen::{
        BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
        KeyUsagePurpose,
    };
    let directory = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    };
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let key = KeyPair::generate().unwrap();
    let ca = params.self_signed(&key).unwrap();
    let issuer = Issuer::new(params, key);
    let ca = write("ca.der", ca.der());
    let leaf = |name: &str| {
        let mut params = CertificateParams::new(vec!["gateway.test".into()]).unwrap();
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];
        let key = KeyPair::generate().unwrap();
        let cert = params.signed_by(&key, &issuer).unwrap();
        (
            write(&format!("{name}.der"), cert.der()),
            write(&format!("{name}.key"), &key.serialize_der()),
        )
    };
    let (server_cert, server_key) = leaf("server");
    let (client_cert, client_key) = leaf("client");
    let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 48,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let prediction = PreparedPrediction::new(&checkpoint, &recipe).unwrap();
    let generated = prediction.prediction().to_vec();
    let mut residual = generated.clone();
    residual[4096..4608].iter_mut().for_each(|b| *b ^= 0x5a);
    let opaque = spiralcore_gateway::measurement::fixture(262_144, "opaque", 811).unwrap();
    let mut edited = opaque.clone();
    edited[8192..8704].iter_mut().for_each(|b| *b ^= 0xa5);
    let checkpoint = write("checkpoint", &checkpoint);
    let recipe = write("recipe", &recipe.encode().unwrap());
    let schema = write(
        "schema",
        &spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n"]).unwrap(),
    );
    let generated = write("generated", &generated);
    let residual = write("residual", &residual);
    let opaque = write("opaque", &opaque);
    let edited = write("edited", &edited);
    let ready = directory.path().join("server-ready.json");
    let mut client = config();
    client["groups"][0]["mode"] = serde_json::json!(mode);
    client["certificate"] = serde_json::json!(client_cert);
    client["private_key"] = serde_json::json!(client_key);
    client["trust_anchor"] = serde_json::json!(ca);
    client["peer_certificate"] = serde_json::json!(server_cert);
    client["store_dir"] = serde_json::json!(directory.path().join("cache"));
    client["ready_path"] = serde_json::json!(directory.path().join("client-ready.json"));
    client["stats_path"] = serde_json::json!(directory.path().join("client-stats.json"));
    client["groups"][0]["checkpoint_path"] = serde_json::json!(checkpoint);
    client["groups"][0]["recipe_path"] = serde_json::json!(recipe);
    client["groups"][0]["schema_path"] = serde_json::json!(schema);
    client["groups"][0]["operations"] = serde_json::json!([
        {"label":"checkpoint","action":"checkpoint","warm_probe":false},
        {"label":"schema","action":"schema","warm_probe":false},
        {"label":"generated","action":"send","path":generated,"warm_probe":true},
        {"label":"residual","action":"send","path":residual,"warm_probe":true},
        {"label":"residual-repeat","action":"send","path":residual,"warm_probe":true},
        {"label":"opaque","action":"send","path":opaque,"warm_probe":true},
        {"label":"opaque-edit","action":"send","path":edited,"warm_probe":true},
        {"label":"opaque-repeat","action":"send","path":edited,"warm_probe":true}
    ]);
    let mut server = client.clone();
    server["role"] = serde_json::json!("listen");
    server["groups"] = serde_json::json!([]);
    server["certificate"] = serde_json::json!(server_cert);
    server["private_key"] = serde_json::json!(server_key);
    server["peer_certificate"] = serde_json::json!(client_cert);
    server["ready_path"] = serde_json::json!(ready);
    server["stats_path"] = serde_json::json!(directory.path().join("server-stats.json"));
    server["verify_files"] = serde_json::json!([generated, residual, opaque, edited]);
    let server = tokio::spawn(run(serde_json::from_value(server).unwrap()));
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    while !ready.exists() {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let ready: serde_json::Value = serde_json::from_slice(&std::fs::read(ready).unwrap()).unwrap();
    client["peer"] = ready["address"].clone();
    tokio::time::timeout_at(deadline, run(serde_json::from_value(client).unwrap()))
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout_at(deadline, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let read = |name| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(directory.path().join(name)).unwrap()).unwrap()
    };
    let client = read("client-stats.json");
    let server = read("server-stats.json");
    assert_eq!(client["status"], "ok");
    assert_eq!(server["status"], "ok");
    assert_eq!(server["readback"]["expected_files_exact"], 4);
    assert_eq!(server["readback"]["reopened"], true);
    assert_eq!(client["operations"][2]["selected_mode"], "RuntimeV1");
    assert_eq!(client["operations"][6]["selected_mode"], "SparseV1");
    for index in [4, 7] {
        assert_eq!(client["operations"][index]["selected_mode"], "Raw");
        assert_eq!(
            client["operations"][index]["publication_path"],
            "verified-cache-hit"
        );
        assert_eq!(client["operations"][index]["data_bytes"], 0);
        assert_eq!(
            client["operations"][index]["selection"]["exact_repeat_probe"],
            true
        );
    }
    assert_eq!(client["namespace"], server["namespace"]);
    let expected_policy = if mode == "combined-cpu" {
        "cpu-conscious"
    } else {
        "exhaustive"
    };
    for operation in client["operations"].as_array().unwrap().iter().skip(2) {
        assert_eq!(operation["selection"]["selection_policy"], expected_policy);
    }
    let unrelated = &client["operations"][5]["selection"];
    if mode == "combined-cpu" {
        assert_eq!(
            unrelated["policy_skipped"],
            serde_json::json!([{
                "mode":"SchemaZstdV1", "reason":"no-useful-template-records",
                "useful_template_records":0
            }])
        );
        assert!(!unrelated["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["mode"] == "SchemaZstdV1"));
        assert!(!unrelated["pruned"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["mode"] == "SchemaZstdV1"));
    } else {
        assert_eq!(unrelated["policy_skipped"], serde_json::json!([]));
    }
    for index in [4, 7] {
        assert_eq!(
            client["operations"][index]["selection"]["policy_skipped"],
            serde_json::json!([])
        );
    }
}
