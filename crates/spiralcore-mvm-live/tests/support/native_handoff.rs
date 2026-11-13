use anyhow::Result;
use std::path::Path;

// The original export releases its temporary directory at successful exit.
pub(super) fn finish(
    _working: tempfile::TempDir,
    _ready: Option<&Path>,
    _archive_sha256: &str,
) -> Result<()> {
    Ok(())
}
