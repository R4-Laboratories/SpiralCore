#[path = "build_support/zstd_probe_guard.rs"]
mod guard;
#[path = "build_support/owned_workspace_layout.rs"]
mod owned_layout;

fn owned_workspace_layout() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=TARGET");
    let qualified = (|| {
        let rustc = std::env::var_os("RUSTC")?;
        let output = std::process::Command::new(rustc)
            .args(["--version", "--verbose"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let version = std::str::from_utf8(&output.stdout).ok()?;
        let value = |prefix: &str| version.lines().find_map(|line| line.strip_prefix(prefix));
        let target = std::env::var("TARGET").ok()?;
        Some(owned_layout::qualified(
            value("release: ")?,
            value("commit-hash: ")?,
            &target,
        ))
    })()
    .unwrap_or(false);
    println!(
        "cargo:rustc-env=SPIRALCORE_OWNED_STD_LAYOUT={}",
        if qualified {
            "qualified-v1"
        } else {
            "unsupported"
        }
    );
}

fn main() {
    owned_workspace_layout();
    println!("cargo:rustc-check-cfg=cfg(spiralcore_bundled_zstd_probe)");
    println!("cargo:rerun-if-env-changed=DEP_ZSTD_INCLUDE");
    for key in [
        "SPIRALCORE_DISABLE_ZSTD_PROBE",
        "ZSTD_SYS_USE_PKG_CONFIG",
        "CRATE_CC_NO_DEFAULTS",
        "CFLAGS",
        "HOST_CFLAGS",
        "TARGET_CFLAGS",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    for target_key in ["TARGET", "HOST"] {
        if let Ok(target) = std::env::var(target_key) {
            println!("cargo:rerun-if-env-changed=CFLAGS_{target}");
            println!(
                "cargo:rerun-if-env-changed=CFLAGS_{}",
                target.replace('-', "_")
            );
        }
    }
    let Some(include) = std::env::var_os("DEP_ZSTD_INCLUDE") else {
        return;
    };
    let include = std::path::Path::new(&include);
    let Some(root) = include.parent().and_then(std::path::Path::parent) else {
        return;
    };
    if include != root.join("zstd/lib") {
        return;
    }
    println!("cargo:rerun-if-changed={}", include.display());
    println!("cargo:rerun-if-changed={}", root.join("build.rs").display());
    if !guard::qualified_sources(root) {
        return;
    }
    println!(
        "cargo:rustc-env=SPIRALCORE_ZSTD_SOURCE_ROOT={}",
        root.display()
    );
    if std::env::vars_os().any(|(key, _)| guard::disables_probe(&key.to_string_lossy())) {
        return;
    }
    println!("cargo:rustc-cfg=spiralcore_bundled_zstd_probe");
}
