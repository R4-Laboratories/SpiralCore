//! Check both terminal archive names for the original source's directed outbox.
//! These names follow archive_path in the pinned guest-agent implementation.
use std::{fs, io, path::Path};

pub fn require_no_directed_archive(slot: &Path, id: &str) -> io::Result<()> {
    let scope = slot
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("outbox-directed-v1-"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| io::Error::other("expected a canonical directed slot"))?;
    for prefix in ["rejected", "uncertain"] {
        let archived = slot.with_file_name(format!("{prefix}-directed-v1-{scope}-{id}"));
        match fs::symlink_metadata(archived) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            Ok(_) => return Err(io::Error::other(format!("original archived as {prefix}"))),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directed_archive_cannot_be_misclassified_as_acceptance() {
        let home = std::env::temp_dir().join(format!("sc-directed-archive-{}", std::process::id()));
        fs::create_dir(&home).unwrap();
        let slot = home.join("outbox-directed-v1-abc");
        assert!(require_no_directed_archive(&slot, "123").is_ok());
        for prefix in ["rejected", "uncertain"] {
            let file = home.join(format!("{prefix}-directed-v1-abc-123"));
            fs::write(&file, [7]).unwrap();
            assert!(!home.join(format!("{prefix}-123")).exists());
            assert!(require_no_directed_archive(&slot, "123").is_err());
            fs::remove_file(file).unwrap();
        }
        fs::remove_dir(home).unwrap();
    }
}
