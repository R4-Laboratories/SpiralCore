//! Execute the actual workflow configuration gates without a hosted runner.
use std::{collections::BTreeMap, fs, path::Path, process::Command};

fn gate(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/workflows/spiralcore-schema-selection.yml");
    let workflow = fs::read_to_string(path).expect("schema WAN workflow must exist");
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
        ("GITHUB_REF", "refs/heads/codex/spiralcore-schema-selection"),
        ("GITHUB_RUN_ATTEMPT", "1"),
        ("GITHUB_SHA", "cf490d3b4acdd9c3a417784042bd8e3a2d9c3640"),
        (
            "SPIRALCORE_SELECTION_SOURCE",
            "cf490d3b4acdd9c3a417784042bd8e3a2d9c3640",
        ),
        (
            "SPIRALCORE_SELECTION_RUN",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        (
            "SELECTION_RECEIVER_SHA",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        ("SELECTION_URL_0", "https://example-zero.trycloudflare.com"),
        ("SELECTION_URL_1", "https://example-one.trycloudflare.com/"),
        ("SELECTION_URL_2", "https://example-two.trycloudflare.com"),
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
fn schema_selection_workflow_accepts_only_the_pinned_first_attempt() {
    assert!(accepted("CONFIG", &[]));
    for (key, value) in [
        ("GITHUB_REF", "refs/heads/main"),
        ("GITHUB_RUN_ATTEMPT", "2"),
        (
            "SPIRALCORE_SELECTION_SOURCE",
            "0000000000000000000000000000000000000000",
        ),
        ("GITHUB_SHA", "CF490D3B4ACDD9C3A417784042BD8E3A2D9C3640"),
        ("SPIRALCORE_SELECTION_RUN", ""),
        ("SELECTION_RECEIVER_SHA", "short"),
    ] {
        assert!(!accepted("CONFIG", &[(key, value)]), "accepted {key}");
    }
}

#[test]
fn schema_selection_workflow_restricts_every_receiver_url() {
    for key in ["SELECTION_URL_0", "SELECTION_URL_1", "SELECTION_URL_2"] {
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
fn schema_selection_workflow_requires_a_full_secret_without_fallback() {
    assert!(accepted("AUTH", &[]));
    for value in [
        "",
        "short",
        "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    ] {
        assert!(!accepted("AUTH", &[("SPIRALCORE_WAN_TOKEN", value)]));
    }
}
