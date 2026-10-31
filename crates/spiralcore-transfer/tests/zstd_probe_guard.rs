#[path = "../build_support/zstd_probe_guard.rs"]
mod guard;

#[test]
fn source_guard_rejects_missing_or_foreign_trees() {
    assert!(!guard::qualified_sources(std::path::Path::new(
        "/spiralcore-nonexistent-zstd-source"
    )));
    assert!(!guard::qualified_sources(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    ))));
}

#[test]
fn system_linkage_and_native_overrides_disable_the_certificate() {
    for key in [
        "SPIRALCORE_DISABLE_ZSTD_PROBE",
        "ZSTD_SYS_USE_PKG_CONFIG",
        "CFLAGS",
        "HOST_CFLAGS",
        "TARGET_CFLAGS",
        "CFLAGS_aarch64_apple_darwin",
        "CFLAGS_x86_64-unknown-linux-gnu",
        "CRATE_CC_NO_DEFAULTS",
    ] {
        assert!(guard::disables_probe(key), "{key}");
    }
    for key in ["PATH", "CARGO_TARGET_DIR", "CARGO_INCREMENTAL"] {
        assert!(!guard::disables_probe(key), "{key}");
    }
}

#[test]
fn qualified_build_records_the_exact_source_tree() {
    if let Some(root) = option_env!("SPIRALCORE_ZSTD_SOURCE_ROOT") {
        assert!(guard::qualified_sources(std::path::Path::new(root)));
        assert_eq!(
            guard::source_digest(std::path::Path::new(root)).unwrap(),
            guard::EXPECTED_SOURCE_DIGEST
        );
    }
}

#[test]
fn qualification_can_require_the_enabled_or_disabled_build_path() {
    match std::env::var("SPIRALCORE_EXPECT_ZSTD_PROBE").as_deref() {
        Ok(expected @ ("enabled" | "disabled")) => {
            let enabled = expected == "enabled";
            assert_eq!(cfg!(spiralcore_bundled_zstd_probe), enabled);
            if enabled {
                assert!(option_env!("SPIRALCORE_ZSTD_SOURCE_ROOT").is_some());
            }
        }
        Err(_) => {}
        Ok(other) => panic!("invalid qualification expectation: {other}"),
    }
}
