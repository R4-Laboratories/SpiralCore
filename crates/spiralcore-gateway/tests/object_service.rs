use spiralcore_gateway::object_service::Config;

fn config() -> serde_json::Value {
    serde_json::json!({
        "role":"listen","bind":"127.0.0.1:0","peer":"127.0.0.1:4433",
        "server_name":"peer.test","certificate":"local.der","private_key":"key.der",
        "trust_anchor":"ca.der","peer_certificate":"peer.der","scope":"tenant-a",
        "policy_epoch":1,"store_dir":"store","stats_path":"stats.json","ready_path":"ready.json",
        "duration_secs":60,"groups":[],"verify_files":[]
    })
}

#[test]
fn object_configuration_is_explicit_bounded_and_role_scoped() {
    let valid = config();
    serde_json::from_value::<Config>(valid.clone())
        .unwrap()
        .validate()
        .unwrap();
    for (field, value) in [
        ("role", serde_json::json!("auto")),
        ("scope", serde_json::json!("")),
        ("duration_secs", serde_json::json!(0)),
        ("stats_path", serde_json::json!("ready.json")),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert!(serde_json::from_value::<Config>(bad)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut bad = valid.clone();
    bad["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Config>(bad).is_err());
    let mut sender = valid;
    sender["role"] = serde_json::json!("connect");
    assert!(serde_json::from_value::<Config>(sender.clone())
        .unwrap()
        .validate()
        .is_err());
    sender["groups"] = serde_json::json!([{"label":"objects","mode":"runtime","checkpoint_path":"checkpoint","recipe_path":"recipe","operations":[{"label":"checkpoint","action":"checkpoint","warm_probe":false},{"label":"object","action":"send","path":"input","warm_probe":true}]}]);
    serde_json::from_value::<Config>(sender.clone())
        .unwrap()
        .validate()
        .unwrap();
    sender["groups"][0]["checkpoint_path"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<Config>(sender)
        .unwrap()
        .validate()
        .is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_and_warm_use_real_gateway_and_reopen_exact_objects() {
    paired_service(None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mismatched_scope_or_policy_epoch_fails_before_opening_the_store() {
    paired_service(Some("scope")).await;
    paired_service(Some("policy_epoch")).await;
}

fn failing_config(dir: &std::path::Path) -> serde_json::Value {
    let mut value = config();
    for field in [
        "certificate",
        "private_key",
        "trust_anchor",
        "peer_certificate",
    ] {
        value[field] = serde_json::json!(dir.join(field));
    }
    value["store_dir"] = serde_json::json!(dir.join("store"));
    value["stats_path"] = serde_json::json!(dir.join("stats.json"));
    value["ready_path"] = serde_json::json!(dir.join("ready.json"));
    value
}

#[tokio::test]
async fn existing_outputs_and_input_aliases_are_preserved_even_when_startup_fails() {
    for output in ["stats_path", "ready_path"] {
        let dir = tempfile::tempdir().unwrap();
        let protected = dir.path().join("private_key");
        std::fs::write(&protected, b"unique original credential or input").unwrap();
        let mut value = failing_config(dir.path());
        value[output] = serde_json::json!(protected);
        assert!(
            spiralcore_gateway::object_service::run(serde_json::from_value(value).unwrap())
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(&protected).unwrap(),
            b"unique original credential or input"
        );
        assert!(
            !dir.path().join("stats.json").exists(),
            "startup refusal must not write another output"
        );
        assert!(!dir.path().join("ready.json").exists());
    }
}

#[tokio::test]
async fn normalized_output_store_and_staging_collisions_fail_without_creating_reports() {
    for collision in ["same", "store", "staging", "lease"] {
        let dir = tempfile::tempdir().unwrap();
        let mut value = failing_config(dir.path());
        match collision {
            "same" => {
                std::fs::create_dir(dir.path().join("sub")).unwrap();
                value["ready_path"] = serde_json::json!(dir.path().join("sub/../stats.json"));
            }
            "store" => value["store_dir"] = value["stats_path"].clone(),
            "staging" => {
                std::fs::write(
                    dir.path().join("stats.json.sc-object.tmp"),
                    b"existing staging input",
                )
                .unwrap();
            }
            _ => {
                std::fs::write(
                    dir.path().join("stats.json.sc-object.lock"),
                    b"another owner's lease",
                )
                .unwrap();
            }
        }
        assert!(
            spiralcore_gateway::object_service::run(serde_json::from_value(value).unwrap())
                .await
                .is_err()
        );
        assert!(
            !dir.path().join("stats.json").exists(),
            "collision {collision} was admitted"
        );
        assert!(!dir.path().join("ready.json").exists());
        if collision == "staging" {
            assert_eq!(
                std::fs::read(dir.path().join("stats.json.sc-object.tmp")).unwrap(),
                b"existing staging input"
            );
        }
        if collision == "lease" {
            assert_eq!(
                std::fs::read(dir.path().join("stats.json.sc-object.lock")).unwrap(),
                b"another owner's lease"
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test]
async fn output_symlink_and_hardlink_aliases_are_not_replaced() {
    for symbolic in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("configuration.json");
        let output = dir.path().join("stats.json");
        std::fs::write(&source, b"original configuration").unwrap();
        if symbolic {
            std::os::unix::fs::symlink(&source, &output).unwrap();
        } else {
            std::fs::hard_link(&source, &output).unwrap();
        }
        let value = failing_config(dir.path());
        assert!(
            spiralcore_gateway::object_service::run(serde_json::from_value(value).unwrap())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&source).unwrap(), b"original configuration");
        assert_eq!(std::fs::read(&output).unwrap(), b"original configuration");
        assert_eq!(
            std::fs::symlink_metadata(&output)
                .unwrap()
                .file_type()
                .is_symlink(),
            symbolic
        );
    }
}

#[tokio::test]
async fn fresh_owned_error_report_is_valid_and_reservations_are_released() {
    let dir = tempfile::tempdir().unwrap();
    let value = failing_config(dir.path());
    assert!(
        spiralcore_gateway::object_service::run(serde_json::from_value(value).unwrap())
            .await
            .is_err()
    );
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("stats.json")).unwrap()).unwrap();
    assert_eq!(report["status"], "error");
    assert_eq!(report["qualified_complete"], false);
    assert!(!dir.path().join("ready.json").exists());
    for suffix in [".sc-object.tmp", ".sc-object.lock"] {
        for output in ["stats.json", "ready.json"] {
            assert!(!dir.path().join(format!("{output}{suffix}")).exists());
        }
    }
}

async fn paired_service(mismatch: Option<&str>) {
    use rcgen::{
        BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
        KeyUsagePurpose,
    };
    use spiralcore_transfer::{
        shared_runtime::{OutputKind, Recipe, State},
        shared_transfer::PreparedPrediction,
    };
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| {
        let p = dir.path().join(name);
        std::fs::write(&p, bytes).unwrap();
        p
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
    let (sc, sk) = leaf("server");
    let (cc, ck) = leaf("client");
    let checkpoint = State::seeded(65).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 81,
        payload: 1.25,
        output: OutputKind::Trace,
    };
    let prediction = PreparedPrediction::new(&checkpoint, &recipe).unwrap();
    let generated = prediction.prediction().to_vec();
    let mut changed = generated.clone();
    changed[900] ^= 55;
    let cp = write("checkpoint", &checkpoint);
    let rp = write("recipe", &recipe.encode().unwrap());
    let gp = write("generated", &generated);
    let ep = write("edited", &changed);
    let base = spiralcore_gateway::measurement::fixture(262144, "opaque", 17).unwrap();
    let mut edit = base.clone();
    edit[8192] ^= 0xa5;
    let bp = write("base", &base);
    let wp = write("warm-edit", &edit);
    let sp = write(
        "schema",
        &spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n"]).unwrap(),
    );
    let mut server = config();
    server["server_name"] = serde_json::json!("gateway.test");
    server["certificate"] = serde_json::json!(sc);
    server["private_key"] = serde_json::json!(sk);
    server["peer_certificate"] = serde_json::json!(cc);
    server["trust_anchor"] = serde_json::json!(ca);
    server["store_dir"] = serde_json::json!(dir.path().join("store"));
    server["stats_path"] = serde_json::json!(dir.path().join("server.json"));
    let ready = dir.path().join("ready-server.json");
    server["ready_path"] = serde_json::json!(ready);
    server["verify_files"] = serde_json::json!([gp, ep, bp, wp]);
    let mut client = server.clone();
    client["role"] = serde_json::json!("connect");
    client["certificate"] = serde_json::json!(cc);
    client["private_key"] = serde_json::json!(ck);
    client["peer_certificate"] = serde_json::json!(sc);
    client["verify_files"] = serde_json::json!([]);
    client["stats_path"] = serde_json::json!(dir.path().join("client.json"));
    client["ready_path"] = serde_json::json!(dir.path().join("ready-client.json"));
    client["groups"] = serde_json::json!([
        {"label":"runtime","mode":"runtime","checkpoint_path":cp,"recipe_path":rp,"operations":[
            {"label":"checkpoint","action":"checkpoint","warm_probe":false},
            {"label":"generated","action":"send","path":gp,"warm_probe":false},
            {"label":"residual","action":"send","path":ep,"warm_probe":false},
            {"label":"generated-cache-no-probe","action":"send","path":gp,"warm_probe":false}]},
        {"label":"warm","mode":"warm","schema_path":sp,"operations":[
            {"label":"schema","action":"schema","warm_probe":false},
            {"label":"seed","action":"seed","path":bp,"warm_probe":false},
            {"label":"sparse","action":"send","path":wp,"warm_probe":false},
            {"label":"repeat","action":"send","path":wp,"warm_probe":true}]}
    ]);
    if let Some(field) = mismatch {
        client[field] = if field == "scope" {
            serde_json::json!("different-authorized-scope")
        } else {
            serde_json::json!(2)
        };
    }
    let server = tokio::spawn(spiralcore_gateway::object_service::run(
        serde_json::from_value(server).unwrap(),
    ));
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready.exists() {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let ready: serde_json::Value = serde_json::from_slice(&std::fs::read(&ready).unwrap()).unwrap();
    client["peer"] = ready["address"].clone();
    let client_result = tokio::time::timeout_at(
        deadline,
        spiralcore_gateway::object_service::run(serde_json::from_value(client).unwrap()),
    )
    .await
    .unwrap();
    let server_result = tokio::time::timeout_at(deadline, server)
        .await
        .unwrap()
        .unwrap();
    if mismatch.is_some() {
        assert!(client_result.is_err(), "mismatched namespace was accepted");
        assert!(server_result.is_err());
        assert!(!dir.path().join("store").exists());
        return;
    }
    client_result.unwrap();
    server_result.unwrap();
    let client: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("client.json")).unwrap()).unwrap();
    let server: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("server.json")).unwrap()).unwrap();
    assert_eq!(server["status"], "ok");
    assert_eq!(server["readback"]["expected_files_exact"], 4);
    assert_eq!(server["readback"]["reopened"], true);
    assert_eq!(client["operations"][1]["selected_mode"], "RuntimeV1");
    assert_eq!(client["operations"][2]["selected_mode"], "RuntimeV1");
    assert_eq!(client["operations"][6]["selected_mode"], "SparseV1");
    assert_eq!(client["operations"][7]["selected_mode"], "Raw");
    assert_eq!(
        client["operations"][1]["publication_path"],
        "verified-transfer"
    );
    assert_eq!(
        client["operations"][2]["publication_path"],
        "verified-transfer"
    );
    assert_eq!(client["operations"][3]["selected_mode"], "RuntimeV1");
    assert_eq!(
        client["operations"][3]["publication_path"],
        "verified-cache-hit"
    );
    assert!(client["operations"][3]["data_bytes"].as_u64().unwrap() > 0);
    assert_eq!(
        client["operations"][7]["publication_path"],
        "verified-cache-hit"
    );
    assert_eq!(client["namespace"], server["namespace"]);
    let operation_ids: std::collections::BTreeSet<_> = client["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|operation| {
            operation["transfer_id"]
                .as_str()
                .expect("operation correlation ID")
        })
        .collect();
    assert_eq!(operation_ids.len(), 8);
    let batch_ids: std::collections::BTreeSet<_> = server["server_batches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|batch| {
            batch["transfer_id"]
                .as_str()
                .expect("receiver correlation ID")
        })
        .collect();
    assert_eq!(operation_ids, batch_ids);
}
