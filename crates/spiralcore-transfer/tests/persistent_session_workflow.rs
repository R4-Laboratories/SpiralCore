//! Execute the actual workflow configuration gates without a hosted runner.
use std::{collections::BTreeMap, fs, path::Path, process::Command};

fn gate(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/workflows/spiralcore-persistent-session.yml");
    let workflow = fs::read_to_string(path).expect("persistent session workflow must exist");
    let start = format!("# BEGIN {name} GATE");
    let end = format!("# END {name} GATE");
    workflow
        .split_once(&start)
        .expect("gate start")
        .1
        .split_once(&end)
        .expect("gate end")
        .0
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn accepted(name: &str, changes: &[(&str, &str)]) -> bool {
    let mut values = BTreeMap::from([
        (
            "GITHUB_REF",
            "refs/heads/codex/spiralcore-persistent-session",
        ),
        ("GITHUB_RUN_ATTEMPT", "1"),
        ("GITHUB_SHA", "cf490d3b4acdd9c3a417784042bd8e3a2d9c3640"),
        (
            "SPIRALCORE_PERSISTENT_SOURCE",
            "cf490d3b4acdd9c3a417784042bd8e3a2d9c3640",
        ),
        (
            "SPIRALCORE_PERSISTENT_RUN",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        (
            "PERSISTENT_RECEIVER_SHA",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        ("PERSISTENT_URL_0", "https://example-zero.trycloudflare.com"),
        ("PERSISTENT_URL_1", "https://example-one.trycloudflare.com/"),
        ("PERSISTENT_URL_2", "https://example-two.trycloudflare.com"),
        (
            "PERSISTENT_URL_3",
            "https://example-three.trycloudflare.com",
        ),
        ("PERSISTENT_URL_4", "https://example-four.trycloudflare.com"),
        ("PERSISTENT_URL_5", "https://example-five.trycloudflare.com"),
        (
            "SPIRALCORE_WAN_TOKEN",
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        ),
    ]);
    values.extend(changes.iter().copied());
    Command::new("/bin/bash")
        .args(["-eu", "-o", "pipefail", "-c", &gate(name)])
        .env_clear()
        .envs(values)
        .output()
        .expect("bash gate runs")
        .status
        .success()
}

#[test]
fn persistent_session_workflow_accepts_only_the_pinned_first_attempt() {
    assert!(accepted("CONFIG", &[]));
    for (key, value) in [
        ("GITHUB_REF", "refs/heads/main"),
        ("GITHUB_RUN_ATTEMPT", "2"),
        (
            "SPIRALCORE_PERSISTENT_SOURCE",
            "0000000000000000000000000000000000000000",
        ),
        ("GITHUB_SHA", "CF490D3B4ACDD9C3A417784042BD8E3A2D9C3640"),
        ("SPIRALCORE_PERSISTENT_RUN", ""),
        ("PERSISTENT_RECEIVER_SHA", "short"),
    ] {
        assert!(!accepted("CONFIG", &[(key, value)]), "accepted {key}");
    }
}

#[test]
fn persistent_session_workflow_restricts_every_receiver_url() {
    for key in [
        "PERSISTENT_URL_0",
        "PERSISTENT_URL_1",
        "PERSISTENT_URL_2",
        "PERSISTENT_URL_3",
        "PERSISTENT_URL_4",
        "PERSISTENT_URL_5",
    ] {
        for value in [
            "",
            "http://example.trycloudflare.com",
            "https://example.com",
            "https://example.trycloudflare.com.attacker.invalid",
            "https://user@example.trycloudflare.com",
            "https://example.trycloudflare.com:443",
            "https://example.trycloudflare.com/path",
            "https://example.trycloudflare.com/?token=x",
            "https://-bad.trycloudflare.com",
            "https://bad-.trycloudflare.com",
        ] {
            assert!(
                !accepted("CONFIG", &[(key, value)]),
                "accepted {key}: {value}"
            );
        }
    }
}

#[test]
fn persistent_session_workflow_requires_a_full_secret_without_fallback() {
    assert!(accepted("AUTH", &[]));
    for value in [
        "",
        "short",
        "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    ] {
        assert!(!accepted("AUTH", &[("SPIRALCORE_WAN_TOKEN", value)]));
    }
}

#[test]
fn persistent_session_workflow_rejects_unknown_purpose() {
    assert!(accepted(
        "CONFIG",
        &[("SPIRALCORE_PERSISTENT_PURPOSE", "pilot")]
    ));
    assert!(accepted(
        "CONFIG",
        &[("SPIRALCORE_PERSISTENT_PURPOSE", "diagnostic")]
    ));
    for value in ["rerun", "DIAGNOSTIC", "diagnostic pilot", "$(echo unsafe)"] {
        assert!(!accepted(
            "CONFIG",
            &[("SPIRALCORE_PERSISTENT_PURPOSE", value)]
        ));
    }
}

#[test]
fn persistent_session_workflow_dose_is_six_or_only_shard_zero() {
    for (purpose, expected) in [
        ("", "pilot\n0 1 2 3 4 5\n"),
        ("pilot", "pilot\n0 1 2 3 4 5\n"),
        ("diagnostic", "diagnostic\n0\n"),
    ] {
        let command = format!(
            "{}\nprintf '%s\\n' \"$PERSISTENT_PURPOSE\" \"$PERSISTENT_SHARDS\"",
            gate("DOSE")
        );
        let output = Command::new("/bin/bash")
            .args(["-eu", "-o", "pipefail", "-c", &command])
            .env_clear()
            .env("SPIRALCORE_PERSISTENT_PURPOSE", purpose)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
    assert!(!accepted(
        "CONFIG",
        &[
            ("SPIRALCORE_PERSISTENT_PURPOSE", "pilot"),
            ("PERSISTENT_URL_5", "")
        ]
    ));
    assert!(accepted(
        "CONFIG",
        &[
            ("SPIRALCORE_PERSISTENT_PURPOSE", "diagnostic"),
            ("PERSISTENT_URL_5", "")
        ]
    ));
    assert!(!accepted(
        "CONFIG",
        &[
            ("SPIRALCORE_PERSISTENT_PURPOSE", "diagnostic"),
            ("PERSISTENT_URL_0", "")
        ]
    ));
}
