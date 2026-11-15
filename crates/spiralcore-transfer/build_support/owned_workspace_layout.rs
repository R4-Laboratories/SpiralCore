//! Qualified std implementations for the owned-store requested-layout model.
pub fn qualified(release: &str, commit: &str, target: &str) -> bool {
    release == "1.97.1"
        && commit == "8bab26f4f68e0e26f0bb7960be334d5b520ea452"
        && matches!(
            target,
            "x86_64-unknown-linux-gnu"
                | "aarch64-unknown-linux-gnu"
                | "x86_64-apple-darwin"
                | "aarch64-apple-darwin"
        )
}
