use std::{path::Path, process::Command};
fn guard() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let text = std::fs::read_to_string(root.join(".github/workflows/spiralcore-wan.yml")).unwrap();
    text.split("      - name: Validate bounded experiment configuration\n")
        .nth(1)
        .unwrap()
        .split("        run: |\n")
        .nth(1)
        .unwrap()
        .split("      - name:")
        .next()
        .unwrap()
        .lines()
        .map(|s| s.strip_prefix("          ").unwrap_or(s))
        .collect::<Vec<_>>()
        .join("\n")
}
#[allow(clippy::too_many_arguments)]
fn allowed(
    qualify: &str,
    suite: &str,
    shard: &str,
    trials: &str,
    url: &str,
    run: &str,
    build: &str,
    artifact: &str,
) -> bool {
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "spiralcore-workflow-guard-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.write_all(b"set -e\n").unwrap();
    file.write_all(guard().as_bytes()).unwrap();
    drop(file);
    let result = Command::new("/usr/bin/env")
        .arg("bash")
        .arg("-e")
        .arg(&path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GITHUB_REF", "refs/heads/codex/spiralcore-efficient")
        .env("SPIRALCORE_WAN_QUALIFY_ONLY", qualify)
        .env("SPIRALCORE_WAN_SUITE", suite)
        .env("SPIRALCORE_WAN_SHARD", shard)
        .env("SPIRALCORE_WAN_TRIALS", trials)
        .env("SPIRALCORE_WAN_URL", url)
        .env("SPIRALCORE_WAN_RUN", run)
        .env("SPIRALCORE_WAN_BUILD_RUN", build)
        .env("SPIRALCORE_WAN_BUILD_ARTIFACT", artifact)
        .status()
        .unwrap()
        .success();
    std::fs::remove_file(path).unwrap();
    result
}
const URL: &str = "https://synthetic-qualification.trycloudflare.com";
const RUN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
#[test]
fn original_eighty_guard_cases_remain_unchanged() {
    let mut count = 0;
    let mut check = |ok: bool, q: &str, s: &str, sh: &str, t: &str, u: &str, r: &str| {
        assert_eq!(allowed(q, s, sh, t, u, r, "", ""), ok, "{q}/{s}/{sh}/{t}");
        count += 1;
    };
    check(true, "1", "", "", "", "", "");
    check(false, "2", "", "", "", "", "");
    check(false, "1", "", "", "30", "", "");
    check(false, "1", "", "", "", URL, "");
    for case in [
        "localized_32",
        "scattered_32x32",
        "prefix_4096",
        "middle_4096",
        "delete_4096",
        "append_4096",
        "truncate_4096",
        "replace_1m",
        "structured_localized_32",
        "compressed_localized_32",
        "cold_random",
        "cold_structured",
    ] {
        for shard in 0..5 {
            check(
                true,
                "",
                &format!("broad16-{case}"),
                &shard.to_string(),
                "30",
                URL,
                RUN,
            );
        }
    }
    for shard in ["", "5", "00", "-1"] {
        check(false, "", "broad16-localized_32", shard, "30", URL, RUN);
    }
    check(false, "", "broad16-unknown", "0", "30", URL, RUN);
    check(false, "", "broad16-localized_32", "0", "1", URL, RUN);
    for shard in 0..3 {
        check(
            true,
            "",
            "large16-localized",
            &shard.to_string(),
            "30",
            URL,
            RUN,
        );
    }
    check(false, "", "large16-localized", "3", "30", URL, RUN);
    for suite in ["efficient-warm", "efficient-prefix"] {
        for trials in ["1", "30"] {
            check(true, "", suite, "", trials, URL, RUN);
        }
        check(false, "", suite, "0", "30", URL, RUN);
    }
    assert_eq!(count, 80);
}
#[test]
fn build_pins_are_paired_numeric_and_forbidden_in_qualification() {
    assert!(allowed(
        "",
        "broad16-localized_32",
        "0",
        "30",
        URL,
        RUN,
        "123",
        "456"
    ));
    for (run, artifact) in [
        ("123", ""),
        ("", "456"),
        ("0", "456"),
        ("01", "456"),
        ("x", "456"),
        ("123", "0"),
        ("123", "01"),
        ("123", "-1"),
        ("18446744073709551616", "456"),
    ] {
        assert!(
            !allowed(
                "",
                "broad16-localized_32",
                "0",
                "30",
                URL,
                RUN,
                run,
                artifact
            ),
            "accepted {run}/{artifact}"
        );
    }
    assert!(!allowed("1", "", "", "", "", "", "123", "456"));
}
