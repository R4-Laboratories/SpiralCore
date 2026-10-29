//! Bounded provenance checking for three exact-source WAN executables.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
pub type Result<T> = std::result::Result<T, String>;
pub const BINARIES: [&str; 3] = [
    "spiralcore-bench",
    "spiralcore-capture",
    "spiralcore-capture-control",
];
const TEXT_LIMIT: u64 = 128 * 1024;
const BINARY_LIMIT: u64 = 32 * 1024 * 1024;
const REPO: &str = "R4-Laboratories/SpiralCore";
const WORKFLOW: &str = ".github/workflows/spiralcore-wan.yml";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub source: String,
    pub repository: String,
    pub workflow: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub compiler: String,
    pub target: String,
    pub features_sha256: String,
    pub lock_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub identity: Identity,
    pub artifact_id: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Qualification {
    pub source_tests_passed: bool,
    pub clippy_passed: bool,
    pub capture_passed: bool,
    pub capture_schema: String,
    pub capture_sha256: String,
    pub capture_controller_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub identity: Identity,
    pub qualification: Qualification,
    pub binaries: Vec<Binary>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunProof {
    pub id: u64,
    pub head_sha: String,
    pub run_attempt: u64,
    pub path: String,
    pub status: String,
    pub conclusion: String,
    pub repository: String,
    pub head_repository: String,
    pub capture_successes: u64,
    pub source_build_successes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactProof {
    pub id: u64,
    pub name: String,
    pub size_in_bytes: u64,
    pub expired: bool,
    pub workflow_run_id: u64,
    pub head_sha: String,
    pub digest: String,
}
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn id(n: u64) -> bool {
    (1..=999_999_999_999_999_999).contains(&n)
}
fn valid_identity(x: &Identity) -> Result<()> {
    require(
        hex(&x.source, 40)
            && x.repository == REPO
            && x.workflow == WORKFLOW
            && id(x.run_id)
            && x.run_attempt == 1,
        "invalid build identity",
    )?;
    require(
        x.compiler.starts_with("rustc 1.97.1 (")
            && x.compiler.len() < 128
            && !x.compiler.contains(['\n', '\r'])
            && x.target == "x86_64-unknown-linux-gnu",
        "unsupported compiler or target",
    )?;
    require(
        hex(&x.features_sha256, 64) && hex(&x.lock_sha256, 64),
        "invalid build input hash",
    )
}
pub fn read_json<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    let bytes = read_bounded(p, TEXT_LIMIT)?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid bounded JSON".into())
}
fn read_bounded(p: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(p).map_err(|_| "input metadata unavailable")?;
    require(
        meta.file_type().is_file() && meta.len() <= limit,
        "nonregular or oversized input",
    )?;
    let f = File::open(p).map_err(|_| "input unavailable")?;
    let mut b = Vec::new();
    f.take(limit + 1)
        .read_to_end(&mut b)
        .map_err(|_| "input read failed")?;
    require(
        b.len() as u64 == meta.len() && b.len() as u64 <= limit,
        "input changed or exceeds bound",
    )?;
    Ok(b)
}
pub fn write_json<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)
        .map_err(|_| "new output required")?;
    serde_json::to_writer_pretty(&mut f, v).map_err(|_| "JSON write failed")?;
    f.write_all(b"\n").map_err(|_| "write failed")?;
    f.sync_all().map_err(|_| "sync failed".into())
}
fn file_hash(p: &Path) -> Result<(u64, String)> {
    file_hash_limit(p, BINARY_LIMIT)
}
fn file_hash_limit(p: &Path, limit: u64) -> Result<(u64, String)> {
    let meta = fs::symlink_metadata(p).map_err(|_| "binary missing")?;
    require(
        meta.file_type().is_file() && meta.len() > 0 && meta.len() <= limit,
        "invalid binary type or size",
    )?;
    let mut f = File::open(p)
        .map_err(|_| "binary open failed")?
        .take(limit + 1);
    let mut h = Sha256::new();
    let mut block = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = f.read(&mut block).map_err(|_| "binary read failed")?;
        if n == 0 {
            break;
        }
        total = total.checked_add(n as u64).ok_or("size overflow")?;
        require(total <= limit, "binary grew beyond bound")?;
        h.update(&block[..n]);
    }
    require(total == meta.len(), "binary changed during verification")?;
    Ok((total, format!("{:x}", h.finalize())))
}
fn files(root: &Path, with_manifest: bool) -> Result<Vec<Binary>> {
    require(
        fs::symlink_metadata(root)
            .map_err(|_| "artifact directory unavailable")?
            .file_type()
            .is_dir(),
        "artifact root must be a real directory",
    )?;
    let mut names = Vec::new();
    for entry in fs::read_dir(root).map_err(|_| "artifact enumeration failed")? {
        let entry = entry.map_err(|_| "artifact entry failed")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid artifact name")?;
        require(names.len() < 4, "too many artifact files")?;
        names.push(name);
    }
    let mut expected = BINARIES.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    if with_manifest {
        expected.push("manifest.json".into())
    }
    names.sort();
    expected.sort();
    require(names == expected, "artifact file set differs")?;
    BINARIES
        .iter()
        .map(|name| {
            let (bytes, sha256) = file_hash(&root.join(name))?;
            Ok(Binary {
                name: (*name).into(),
                bytes,
                sha256,
            })
        })
        .collect()
}
pub fn create(expected: &Expected, root: &Path, capture: &Path) -> Result<Manifest> {
    valid_identity(&expected.identity)?;
    require(
        expected.artifact_id.is_none(),
        "producer cannot select an existing artifact",
    )?;
    #[derive(Deserialize)]
    struct Capture {
        schema: String,
        passed: bool,
        controller_sha256: String,
    }
    let bytes = read_bounded(capture, 16 * 1024 * 1024)?;
    let c: Capture = serde_json::from_slice(&bytes).map_err(|_| "invalid capture qualification")?;
    require(
        c.schema == "spiralcore.capture-qualification.v2" && c.passed,
        "capture qualification not passed",
    )?;
    let binaries = files(root, false)?;
    require(
        c.controller_sha256 == binaries[2].sha256,
        "capture qualification used a different controller",
    )?;
    Ok(Manifest {
        schema: "spiralcore.wan-sender-build.v1".into(),
        identity: expected.identity.clone(),
        qualification: Qualification {
            source_tests_passed: true,
            clippy_passed: true,
            capture_passed: true,
            capture_schema: c.schema,
            capture_sha256: format!("{:x}", Sha256::digest(bytes)),
            capture_controller_sha256: c.controller_sha256,
        },
        binaries,
    })
}
pub fn check_proofs(expected: &Expected, run: &RunProof, artifact: &ArtifactProof) -> Result<()> {
    valid_identity(&expected.identity)?;
    let artifact_id = expected
        .artifact_id
        .filter(|v| id(*v))
        .ok_or("an explicit artifact ID is required")?;
    let x = &expected.identity;
    require(
        run.id == x.run_id
            && run.head_sha == x.source
            && run.run_attempt == 1
            && run.path == x.workflow
            && run.status == "completed"
            && run.conclusion == "success"
            && run.repository == x.repository
            && run.head_repository == x.repository
            && run.capture_successes == 1
            && run.source_build_successes == 1,
        "producer run not qualified for this source",
    )?;
    require(
        artifact.id == artifact_id
            && artifact.name == format!("spiralcore-wan-sender-{}", x.source)
            && !artifact.expired
            && artifact.workflow_run_id == x.run_id
            && artifact.head_sha == x.source
            && artifact.size_in_bytes > 0
            && artifact.size_in_bytes <= 100 * 1024 * 1024
            && artifact
                .digest
                .strip_prefix("sha256:")
                .is_some_and(|s| hex(s, 64)),
        "artifact identity, lifetime or bound differs",
    )?;
    Ok(())
}
pub fn verify(
    expected: &Expected,
    run: &RunProof,
    artifact: &ArtifactProof,
    root: &Path,
) -> Result<Manifest> {
    check_proofs(expected, run, artifact)?;
    let x = &expected.identity;
    let manifest: Manifest = read_json(&root.join("manifest.json"))?;
    require(
        manifest.schema == "spiralcore.wan-sender-build.v1" && manifest.identity == *x,
        "manifest build inputs differ",
    )?;
    let q = &manifest.qualification;
    require(
        q.source_tests_passed
            && q.clippy_passed
            && q.capture_passed
            && q.capture_schema == "spiralcore.capture-qualification.v2"
            && hex(&q.capture_sha256, 64)
            && manifest
                .binaries
                .get(2)
                .is_some_and(|b| b.sha256 == q.capture_controller_sha256),
        "manifest qualification not passed",
    )?;
    require(
        manifest.binaries == files(root, true)?,
        "binary lengths or hashes differ",
    )?;
    Ok(manifest)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: std::path::PathBuf,
        expected: Expected,
        run: RunProof,
        artifact: ArtifactProof,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "spiralcore-build-manifest-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let identity = Identity {
                source: "a".repeat(40),
                repository: REPO.into(),
                workflow: WORKFLOW.into(),
                run_id: 123,
                run_attempt: 1,
                compiler: "rustc 1.97.1 (fixture 2026-01-01)".into(),
                target: "x86_64-unknown-linux-gnu".into(),
                features_sha256: "b".repeat(64),
                lock_sha256: "c".repeat(64),
            };
            for name in BINARIES {
                fs::write(root.join(name), name.as_bytes()).unwrap();
            }
            let manifest = Manifest {
                schema: "spiralcore.wan-sender-build.v1".into(),
                identity: identity.clone(),
                qualification: Qualification {
                    source_tests_passed: true,
                    clippy_passed: true,
                    capture_passed: true,
                    capture_schema: "spiralcore.capture-qualification.v2".into(),
                    capture_sha256: "d".repeat(64),
                    capture_controller_sha256: format!(
                        "{:x}",
                        Sha256::digest(BINARIES[2].as_bytes())
                    ),
                },
                binaries: files(&root, false).unwrap(),
            };
            write_json(&root.join("manifest.json"), &manifest).unwrap();
            Self {
                root,
                expected: Expected {
                    identity,
                    artifact_id: Some(456),
                },
                run: RunProof {
                    id: 123,
                    head_sha: "a".repeat(40),
                    run_attempt: 1,
                    path: WORKFLOW.into(),
                    status: "completed".into(),
                    conclusion: "success".into(),
                    repository: REPO.into(),
                    head_repository: REPO.into(),
                    capture_successes: 1,
                    source_build_successes: 1,
                },
                artifact: ArtifactProof {
                    id: 456,
                    name: format!("spiralcore-wan-sender-{}", "a".repeat(40)),
                    size_in_bytes: 2048,
                    expired: false,
                    workflow_run_id: 123,
                    head_sha: "a".repeat(40),
                    digest: format!("sha256:{}", "e".repeat(64)),
                },
            }
        }
        fn check(&self) -> Result<Manifest> {
            verify(&self.expected, &self.run, &self.artifact, &self.root)
        }
        fn mutate(&self, f: impl FnOnce(&mut Manifest)) {
            let p = self.root.join("manifest.json");
            let mut m: Manifest = read_json(&p).unwrap();
            f(&mut m);
            fs::write(p, serde_json::to_vec(&m).unwrap()).unwrap();
        }
    }
    #[test]
    fn accepts_only_exact_qualified_build() {
        assert!(Fixture::new().check().is_ok());
    }
    #[test]
    fn rejects_corrupt_binary_before_execution() {
        let f = Fixture::new();
        fs::write(f.root.join(BINARIES[0]), b"corrupt").unwrap();
        assert!(f.check().is_err());
    }
    #[test]
    fn rejects_wrong_manifest_identity_and_qualification() {
        for k in 0..12 {
            let f = Fixture::new();
            f.mutate(|m| match k {
                0 => m.schema.push('x'),
                1 => m.identity.source = "b".repeat(40),
                2 => m.identity.repository = "fork/repo".into(),
                3 => m.identity.workflow = "other".into(),
                4 => m.identity.compiler = "rustc other".into(),
                5 => m.identity.target = "aarch64-apple-darwin".into(),
                6 => m.identity.features_sha256 = "d".repeat(64),
                7 => m.identity.lock_sha256 = "d".repeat(64),
                8 => m.qualification.source_tests_passed = false,
                9 => m.qualification.clippy_passed = false,
                10 => m.qualification.capture_passed = false,
                _ => m.identity.run_attempt = 2,
            });
            assert!(f.check().is_err(), "case{k}");
        }
    }
    #[test]
    fn rejects_wrong_run_proof() {
        for k in 0..11 {
            let mut f = Fixture::new();
            match k {
                0 => f.run.id += 1,
                1 => f.run.head_sha = "b".repeat(40),
                2 => f.run.run_attempt = 2,
                3 => f.run.path = "other".into(),
                4 => f.run.status = "in_progress".into(),
                5 => f.run.conclusion = "failure".into(),
                6 => f.run.repository = "fork/repo".into(),
                7 => f.run.head_repository = "fork/repo".into(),
                8 => f.run.capture_successes = 0,
                9 => f.run.capture_successes = 2,
                _ => f.run.source_build_successes = 0,
            };
            assert!(f.check().is_err(), "case{k}");
        }
    }
    #[test]
    fn rejects_wrong_expired_or_oversized_artifact() {
        for k in 0..8 {
            let mut f = Fixture::new();
            match k {
                0 => f.artifact.id += 1,
                1 => f.artifact.name.push('x'),
                2 => f.artifact.expired = true,
                3 => f.artifact.workflow_run_id += 1,
                4 => f.artifact.head_sha = "b".repeat(40),
                5 => f.artifact.size_in_bytes = 100 * 1024 * 1024 + 1,
                6 => f.artifact.digest = "wrong".into(),
                _ => f.expected.artifact_id = None,
            };
            assert!(f.check().is_err(), "case{k}");
        }
    }
    #[test]
    fn rejects_extra_missing_symlink_and_oversized_files() {
        for k in 0..4 {
            let f = Fixture::new();
            let p = f.root.join(BINARIES[0]);
            match k {
                0 => fs::write(f.root.join("secret"), b"no").unwrap(),
                1 => fs::remove_file(p).unwrap(),
                2 => {
                    fs::remove_file(&p).unwrap();
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(BINARIES[1], p).unwrap();
                }
                _ => File::create(p).unwrap().set_len(BINARY_LIMIT + 1).unwrap(),
            };
            assert!(f.check().is_err(), "case{k}");
        }
    }
    #[test]
    fn typed_json_rejects_unknown_duplicate_trailing_and_oversized() {
        for k in 0..4 {
            let f = Fixture::new();
            let p = f.root.join("manifest.json");
            let mut b = fs::read(&p).unwrap();
            match k {
                0 => {
                    b.pop();
                    b.extend_from_slice(b",\"unknown\":true}");
                }
                1 => {
                    let old = String::from_utf8(b).unwrap();
                    b = old
                        .replacen("{", "{\"schema\":\"duplicate\",", 1)
                        .into_bytes();
                }
                2 => b.extend_from_slice(b"{}"),
                _ => b = vec![b' '; TEXT_LIMIT as usize + 1],
            };
            fs::write(p, b).unwrap();
            assert!(f.check().is_err());
        }
    }
}

// Bound the raw footer count before the ZIP library allocates its entry collection.
// ZIP64 is accepted only with one disk, exactly four entries and a bounded central
// directory. Cross-check the library's chosen offset so a second footer is not trusted.
fn central_directory_bound(file: &mut File, len: u64) -> Result<u64> {
    let n = len.min(65557 + 20);
    file.seek(SeekFrom::Start(len - n))
        .map_err(|_| "ZIP seek failed")?;
    let mut tail = vec![0u8; n as usize];
    file.read_exact(&mut tail)
        .map_err(|_| "ZIP footer read failed")?;
    let u16_at = |b: &[u8], i: usize| u16::from_le_bytes(b[i..i + 2].try_into().unwrap());
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    let u64_at = |b: &[u8], i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap());
    require(tail.len() >= 22, "ZIP footer missing")?;
    let at = (0..=tail.len() - 22)
        .rev()
        .find(|&i| {
            &tail[i..i + 4] == b"PK\x05\x06"
                && i + 22 + u16_at(&tail, i + 20) as usize == tail.len()
        })
        .ok_or("ZIP footer missing")?;
    let footer = &tail[at..];
    require(
        u16_at(footer, 4) == 0 && u16_at(footer, 6) == 0,
        "multi-disk ZIP rejected",
    )?;
    let mut count = u16_at(footer, 10) as u64;
    let mut disk_count = u16_at(footer, 8) as u64;
    let mut size = u32_at(footer, 12) as u64;
    let mut start = u32_at(footer, 16) as u64;
    let mut end = len - n + at as u64;
    if at >= 20 && &tail[at - 20..at - 16] == b"PK\x06\x07" {
        let locator = &tail[at - 20..at];
        require(
            u32_at(locator, 4) == 0 && u32_at(locator, 16) == 1,
            "ZIP64 disks rejected",
        )?;
        let pos = u64_at(locator, 8);
        require(
            pos.checked_add(56).is_some_and(|p| p <= end - 20),
            "ZIP64 footer range invalid",
        )?;
        file.seek(SeekFrom::Start(pos))
            .map_err(|_| "ZIP64 seek failed")?;
        let mut z = [0u8; 56];
        file.read_exact(&mut z).map_err(|_| "ZIP64 read failed")?;
        require(
            &z[..4] == b"PK\x06\x06" && u32_at(&z, 16) == 0 && u32_at(&z, 20) == 0,
            "ZIP64 footer invalid",
        )?;
        let record = u64_at(&z, 4);
        require(
            (44..=4096).contains(&record)
                && pos.checked_add(12).and_then(|v| v.checked_add(record)) == Some(end - 20),
            "ZIP64 footer length invalid",
        )?;
        disk_count = u64_at(&z, 24);
        count = u64_at(&z, 32);
        size = u64_at(&z, 40);
        start = u64_at(&z, 48);
        end = pos;
    }
    require(
        count == 4
            && disk_count == 4
            && size <= 65536
            && start.checked_add(size).is_some_and(|v| v <= end),
        "ZIP central directory count or bound invalid",
    )?;
    Ok(start)
}
pub fn verify_archive(artifact: &ArtifactProof, path: &Path) -> Result<serde_json::Value> {
    let (bytes, sha256) = file_hash_limit(path, 100 * 1024 * 1024)?;
    require(
        bytes == artifact.size_in_bytes && artifact.digest == format!("sha256:{sha256}"),
        "archive size or authenticated digest differs",
    )?;
    let mut file = File::open(path).map_err(|_| "archive open failed")?;
    let central = central_directory_bound(&mut file, bytes)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| "invalid ZIP archive")?;
    require(
        archive.len() == 4 && archive.offset() == 0 && archive.central_directory_start() == central,
        "ZIP entry count or offset differs",
    )?;
    let mut names = std::collections::BTreeSet::new();
    let mut entries = Vec::new();
    let mut expanded = 0u64;
    for i in 0..4 {
        let entry = archive.by_index_raw(i).map_err(|_| "invalid ZIP entry")?;
        let name = std::str::from_utf8(entry.name_raw()).map_err(|_| "invalid ZIP filename")?;
        require(
            (BINARIES.contains(&name) || name == "manifest.json") && names.insert(name.to_string()),
            "unexpected or duplicate ZIP filename",
        )?;
        require(
            entry.is_file()
                && !entry.is_symlink()
                && !entry.encrypted()
                && entry
                    .unix_mode()
                    .is_none_or(|m| m & 0o170000 == 0 || m & 0o170000 == 0o100000),
            "nonregular or encrypted ZIP entry",
        )?;
        require(
            matches!(
                entry.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::DEFLATE
            ),
            "unsupported ZIP compression",
        )?;
        require(
            entry
                .data_start()
                .and_then(|start| start.checked_add(entry.compressed_size()))
                .is_some_and(|end| end <= central),
            "ZIP compressed range exceeds bound",
        )?;
        let limit = if name == "manifest.json" {
            TEXT_LIMIT
        } else {
            BINARY_LIMIT
        };
        require(
            entry.size() > 0 && entry.size() <= limit,
            "ZIP expanded entry exceeds bound",
        )?;
        expanded = expanded
            .checked_add(entry.size())
            .ok_or("ZIP expansion overflow")?;
        entries.push(serde_json::json!({"name":name,"bytes":entry.size(),"crc32":entry.crc32()}));
    }
    require(
        expanded <= 3 * BINARY_LIMIT + TEXT_LIMIT,
        "ZIP total expansion exceeds bound",
    )?;
    require(
        !archive
            .has_overlapping_files()
            .map_err(|_| "invalid ZIP data ranges")?,
        "overlapping ZIP entries",
    )?;
    Ok(
        serde_json::json!({"schema":"spiralcore.sender-archive-validation.v1","status":"passed","artifact_id":artifact.id,"archive_bytes":bytes,"archive_sha256":sha256,"expanded_bytes":expanded,"entries":entries,"scope":"authenticated archive digest and bounded flat regular entries checked before extraction; manifest and every extracted binary still require verification"}),
    )
}
#[cfg(test)]
mod archive_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Archive {
        path: std::path::PathBuf,
        proof: ArtifactProof,
    }
    impl Drop for Archive {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }
    impl Archive {
        fn new(names: &[&str]) -> Self {
            let path = std::env::temp_dir().join(format!(
                "spiralcore-build-archive-{}-{}.zip",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .read(true)
                .open(&path)
                .unwrap();
            let mut zip = zip::ZipWriter::new(file);
            for name in names {
                zip.start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
                zip.write_all(b"fixture").unwrap();
            }
            zip.finish().unwrap();
            let bytes = fs::read(&path).unwrap();
            let proof = ArtifactProof {
                id: 456,
                name: "fixture".into(),
                size_in_bytes: bytes.len() as u64,
                expired: false,
                workflow_run_id: 123,
                head_sha: "a".repeat(40),
                digest: format!("sha256:{:x}", Sha256::digest(bytes)),
            };
            Self { path, proof }
        }
        fn valid() -> Self {
            Self::new(&[BINARIES[0], BINARIES[1], BINARIES[2], "manifest.json"])
        }
    }
    #[test]
    fn accepts_exact_four_bounded_regular_entries() {
        let a = Archive::valid();
        assert!(verify_archive(&a.proof, &a.path).is_ok());
    }
    #[test]
    fn rejects_changed_archive_before_container_parsing() {
        let mut a = Archive::valid();
        a.proof.digest = format!("sha256:{}", "0".repeat(64));
        assert!(verify_archive(&a.proof, &a.path).is_err());
    }
    #[test]
    fn rejects_extra_missing_or_traversal_entries() {
        for names in [
            vec![
                BINARIES[0],
                BINARIES[1],
                BINARIES[2],
                "manifest.json",
                "extra",
            ],
            vec![BINARIES[0]],
            vec![BINARIES[0], BINARIES[1], BINARIES[2], "../manifest.json"],
        ] {
            let a = Archive::new(&names);
            assert!(verify_archive(&a.proof, &a.path).is_err());
        }
    }
    #[test]
    fn rejects_oversized_declared_expansion() {
        let mut a = Archive::valid();
        let mut bytes = fs::read(&a.path).unwrap();
        let start = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        bytes[start + 24..start + 28].copy_from_slice(&((BINARY_LIMIT + 1) as u32).to_le_bytes());
        fs::write(&a.path, &bytes).unwrap();
        a.proof.digest = format!("sha256:{:x}", Sha256::digest(bytes));
        assert!(verify_archive(&a.proof, &a.path).is_err());
    }
    #[test]
    fn zip64_four_entry_footer_is_accepted_but_huge_counts_are_rejected() {
        let mut a = Archive::valid();
        let original = fs::read(&a.path).unwrap();
        let e = original.len() - 22;
        let mut z = vec![0u8; 56];
        z[..4].copy_from_slice(b"PK\x06\x06");
        z[4..12].copy_from_slice(&44u64.to_le_bytes());
        z[12..14].copy_from_slice(&45u16.to_le_bytes());
        z[14..16].copy_from_slice(&45u16.to_le_bytes());
        z[24..32].copy_from_slice(&4u64.to_le_bytes());
        z[32..40].copy_from_slice(&4u64.to_le_bytes());
        z[40..48].copy_from_slice(
            &(u32::from_le_bytes(original[e + 12..e + 16].try_into().unwrap()) as u64)
                .to_le_bytes(),
        );
        z[48..56].copy_from_slice(
            &(u32::from_le_bytes(original[e + 16..e + 20].try_into().unwrap()) as u64)
                .to_le_bytes(),
        );
        let mut locator = vec![0u8; 20];
        locator[..4].copy_from_slice(b"PK\x06\x07");
        locator[8..16].copy_from_slice(&(e as u64).to_le_bytes());
        locator[16..20].copy_from_slice(&1u32.to_le_bytes());
        let mut bytes = original[..e].to_vec();
        bytes.extend_from_slice(&z);
        bytes.extend_from_slice(&locator);
        bytes.extend_from_slice(&original[e..]);
        a.proof.size_in_bytes = bytes.len() as u64;
        a.proof.digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        fs::write(&a.path, &bytes).unwrap();
        assert!(verify_archive(&a.proof, &a.path).is_ok());
        bytes[e + 32..e + 40].copy_from_slice(&u64::MAX.to_le_bytes());
        a.proof.digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        fs::write(&a.path, bytes).unwrap();
        assert!(verify_archive(&a.proof, &a.path).is_err());
    }
    #[test]
    fn symlink_archive_entry_is_rejected_before_extraction() {
        let mut a = Archive::valid();
        let mut bytes = fs::read(&a.path).unwrap();
        let c = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        bytes[c + 5] = 3;
        bytes[c + 38..c + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        a.proof.digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        fs::write(&a.path, bytes).unwrap();
        assert!(verify_archive(&a.proof, &a.path).is_err());
    }
}
