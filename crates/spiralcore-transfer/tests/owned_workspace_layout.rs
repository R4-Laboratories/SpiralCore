#[path = "../build_support/owned_workspace_layout.rs"]
mod layout;

#[test]
fn unqualified_std_release_commit_or_target_cannot_enable_the_workspace_model() {
    const COMMIT: &str = "8bab26f4f68e0e26f0bb7960be334d5b520ea452";
    for target in [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ] {
        assert!(layout::qualified("1.97.1", COMMIT, target));
        assert!(!layout::qualified("1.98.0", COMMIT, target));
        assert!(!layout::qualified("1.97.1", "unknown", target));
        assert!(!layout::qualified("1.97.1-nightly", COMMIT, target));
    }
    for target in [
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
        "x86_64-unknown-linux-musl",
        "unknown",
    ] {
        assert!(!layout::qualified("1.97.1", COMMIT, target));
    }
}
