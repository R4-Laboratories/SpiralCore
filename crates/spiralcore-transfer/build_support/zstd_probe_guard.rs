use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

// zstd-sys 2.1.2+zstd.1.5.7: sorted bundled C/headers and its build script.
pub const EXPECTED_SOURCE_DIGEST: [u8; 32] = [
    0x64, 0x34, 0xf9, 0xd7, 0x4e, 0xc3, 0xcf, 0xb7, 0xfb, 0xdf, 0x62, 0x42, 0x07, 0x16, 0xc8, 0x91,
    0x24, 0x51, 0xc0, 0x46, 0x80, 0x5f, 0x93, 0x2e, 0x4b, 0x4b, 0xa7, 0xd0, 0x2b, 0x28, 0x34, 0xde,
];

pub fn disables_probe(key: &str) -> bool {
    matches!(
        key,
        "SPIRALCORE_DISABLE_ZSTD_PROBE"
            | "ZSTD_SYS_USE_PKG_CONFIG"
            | "CRATE_CC_NO_DEFAULTS"
            | "CFLAGS"
    ) || key.starts_with("CFLAGS_")
        || key.ends_with("_CFLAGS")
}

pub fn qualified_sources(root: &Path) -> bool {
    source_digest(root).is_ok_and(|digest| digest == EXPECTED_SOURCE_DIGEST)
}

pub fn source_digest(root: &Path) -> io::Result<[u8; 32]> {
    let mut files = vec!["build.rs".to_owned()];
    collect(root, Path::new("zstd/lib"), &mut files)?;
    files.sort();
    let mut hash = Sha256::new();
    hash.update(b"spiralcore.zstd.bundled-source.v1\0");
    for name in files {
        let bytes = fs::read(root.join(&name))?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    Ok(hash.finalize().into())
}

fn collect(root: &Path, relative: &Path, files: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = relative.join(entry.file_name());
        if kind.is_symlink() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "source symlink"));
        }
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if kind.is_file()
            && matches!(path.extension().and_then(|v| v.to_str()), Some("c" | "h"))
        {
            let name = path
                .to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "source path"))?;
            files.push(name.replace('\\', "/"));
        }
    }
    Ok(())
}
