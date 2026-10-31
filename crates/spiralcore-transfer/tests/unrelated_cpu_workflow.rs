//! Execute the actual prospective WAN configuration gates without a runner.
use std::{collections::BTreeMap, fs, path::Path, process::Command};

fn workflow() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.github/workflows/spiralcore-unrelated-cpu.yml"),
    )
    .expect("unrelated CPU workflow must exist")
}

fn gate(name: &str) -> String {
    let source = workflow();
    source
        .split_once(&format!("# BEGIN {name} GATE"))
        .expect("gate start")
        .1
        .split_once(&format!("# END {name} GATE"))
        .expect("gate end")
        .0
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run(name: &str, changes: &[(&str, &str)], suffix: &str) -> std::process::Output {
    let mut values = BTreeMap::from([
        ("GITHUB_REF", "refs/heads/codex/spiralcore-unrelated-cpu"),
        ("GITHUB_RUN_ATTEMPT", "1"),
        ("GITHUB_SHA", "56e15db5c77fe2ecdc03476d8e09664cd4213fd1"),
        (
            "SPIRALCORE_UNRELATED_SOURCE",
            "56e15db5c77fe2ecdc03476d8e09664cd4213fd1",
        ),
        (
            "SPIRALCORE_UNRELATED_RUN",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        (
            "UNRELATED_RECEIVER_SHA",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        ("UNRELATED_URL_0", "https://example-zero.trycloudflare.com"),
        ("UNRELATED_URL_1", "https://example-one.trycloudflare.com/"),
        ("UNRELATED_URL_2", "https://example-two.trycloudflare.com"),
        ("UNRELATED_URL_3", "https://example-three.trycloudflare.com"),
        ("UNRELATED_URL_4", "https://example-four.trycloudflare.com"),
        ("UNRELATED_URL_5", "https://example-five.trycloudflare.com"),
        (
            "SPIRALCORE_WAN_TOKEN",
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        ),
    ]);
    values.extend(changes.iter().copied());
    Command::new("/bin/bash")
        .args([
            "-eu",
            "-o",
            "pipefail",
            "-c",
            &format!("{}\n{suffix}", gate(name)),
        ])
        .env_clear()
        .envs(values)
        .output()
        .expect("bash gate runs")
}

#[test]
fn unrelated_workflow_requires_exact_source_branch_and_first_attempt() {
    assert!(run("CONFIG", &[], "").status.success());
    for change in [
        ("GITHUB_REF", "refs/heads/main"),
        ("GITHUB_RUN_ATTEMPT", "2"),
        ("GITHUB_SHA", "56E15DB5C77FE2ECDC03476D8E09664CD4213FD1"),
        (
            "SPIRALCORE_UNRELATED_SOURCE",
            "0000000000000000000000000000000000000000",
        ),
        ("SPIRALCORE_UNRELATED_RUN", ""),
        ("UNRELATED_RECEIVER_SHA", "short"),
    ] {
        assert!(!run("CONFIG", &[change], "").status.success(), "{change:?}");
    }
}

#[test]
fn unrelated_workflow_restricts_all_six_receiver_origins() {
    for key in [
        "UNRELATED_URL_0",
        "UNRELATED_URL_1",
        "UNRELATED_URL_2",
        "UNRELATED_URL_3",
        "UNRELATED_URL_4",
        "UNRELATED_URL_5",
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
                !run("CONFIG", &[(key, value)], "").status.success(),
                "{key}: {value}"
            );
        }
    }
}

#[test]
fn unrelated_workflow_requires_the_complete_capability() {
    assert!(run("AUTH", &[], "").status.success());
    for value in [
        "",
        "short",
        "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    ] {
        assert!(!run("AUTH", &[("SPIRALCORE_WAN_TOKEN", value)], "")
            .status
            .success());
    }
}

#[test]
fn unrelated_workflow_fixes_one_six_shard_pilot_and_storage_cap() {
    let output = run(
        "DOSE",
        &[("UNRELATED_SHARDS", "0 1 2 3 4 5 6"), ("UNRELATED_MAX_TRANSFERS", "9999")],
        "printf '%s\\n' \"$UNRELATED_SHARDS\" \"$UNRELATED_TRIALS_PER_SIZE\" \"$UNRELATED_MAX_TRANSFERS_PER_SHARD\" \"$UNRELATED_MAX_TRANSFERS\" \"$UNRELATED_MAX_STORE_BYTES\"",
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0 1 2 3 4 5\n6\n70\n420\n805306368\n"
    );
    let source = workflow();
    assert!(source.contains("local_advancement:false"));
    assert!(source.contains(
        "designation:\"one six-pair WAN diagnostic; local gates failed; not accepted\""
    ));
}

#[test]
fn unrelated_workflow_pins_overlay_restore_and_always_retains_failures() {
    let source = workflow();
    assert!(source.contains("baseline=56e15db5c77fe2ecdc03476d8e09664cd4213fd1"));
    assert!(source.contains("for file in src/delta.rs src/schema.rs Cargo.toml Cargo.lock; do"));
    assert!(source.contains("git show \"$baseline:$crate/$file\""));
    assert!(source.contains("git show \"$GITHUB_SHA:$crate/$file\""));
    assert!(source.contains("del(.delta,.schema,.cargo_manifest,.lockfile)"));
    assert!(source.contains("sha256sum -c unrelated-cpu/source-files.sha256"));
    assert!(
        source.contains("sha256sum unrelated-cpu/frozen/baseline unrelated-cpu/frozen/candidate")
    );
    assert!(source.contains("persist-credentials: false"));
    assert!(source.contains("contents: read"));
    assert!(source.contains("cancel-in-progress: false"));
    assert!(source.matches("if: always()").count() >= 2);
    assert!(!source.contains("workflow_dispatch") && !source.contains("pull_request"));
}
