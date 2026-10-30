use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, String>;
const CAP: u64 = 256 * 1024 * 1024;
const SOURCE: &str = "9dce4c57d1f41cef7365b297cd690849afce34e7";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn require(b: bool, why: &str) -> Result<()> {
    if b {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn hash_file(path: &Path, limit: u64) -> Result<(u64, String)> {
    require(
        fs::symlink_metadata(path)
            .map_err(err)?
            .file_type()
            .is_file(),
        "not regular file",
    )?;
    let mut f = File::open(path).map_err(err)?;
    let mut hash = Sha256::new();
    let mut n = 0u64;
    let mut buf = [0u8; 65536];
    loop {
        let k = f.read(&mut buf).map_err(err)?;
        if k == 0 {
            break;
        }
        n = n.checked_add(k as u64).ok_or("byte overflow")?;
        require(n <= limit, "file exceeds cap")?;
        hash.update(&buf[..k]);
    }
    Ok((n, format!("{:x}", hash.finalize())))
}
fn text(path: &Path) -> Result<String> {
    let mut f = File::open(path).map_err(err)?.take(1024 * 1024 + 1);
    let mut b = Vec::new();
    f.read_to_end(&mut b).map_err(err)?;
    require(b.len() <= 1024 * 1024, "text cap")?;
    String::from_utf8(b).map_err(err)
}
fn json_file(path: &Path) -> Result<Value> {
    serde_json::from_str(&text(path)?).map_err(err)
}
fn safe_name(name: &str) -> Result<()> {
    require(
        !name.is_empty()
            && name.len() <= 512
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b)),
        "invalid member name",
    )?;
    require(
        Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
            && !name.contains("//")
            && !name.ends_with('/'),
        "unsafe path",
    )
}
fn inventory(root: &Path) -> Result<BTreeSet<String>> {
    fn visit(root: &Path, dir: &Path, depth: usize, out: &mut BTreeSet<String>) -> Result<()> {
        require(depth <= 8, "directory depth")?;
        require(
            fs::symlink_metadata(dir).map_err(err)?.file_type().is_dir(),
            "directory is not plain",
        )?;
        for entry in fs::read_dir(dir).map_err(err)? {
            let entry = entry.map_err(err)?;
            let p = entry.path();
            let t = entry.file_type().map_err(err)?;
            if t.is_dir() {
                visit(root, &p, depth + 1, out)?;
            } else {
                require(t.is_file(), "nonregular remote entry")?;
                let name = p
                    .strip_prefix(root)
                    .map_err(err)?
                    .to_str()
                    .ok_or("path utf8")?
                    .to_owned();
                safe_name(&name)?;
                require(out.insert(name), "duplicate remote entry")?;
                require(out.len() <= 64, "inventory cap")?;
            }
        }
        Ok(())
    }
    let mut out = BTreeSet::new();
    visit(root, root, 0, &mut out)?;
    Ok(out)
}
fn entries(archive: &Path, expected: usize) -> Result<Vec<(String, u64)>> {
    require(
        (1..=64).contains(&expected),
        "expected count must be positive and bounded",
    )?;
    require(
        fs::metadata(archive).map_err(err)?.len() <= CAP,
        "archive cap",
    )?;
    let mut z = zip::ZipArchive::new(File::open(archive).map_err(err)?).map_err(err)?;
    require(
        z.len() == expected,
        "archive count mismatch or empty iteration",
    )?;
    let mut out = Vec::new();
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for index in 0..z.len() {
        let f = z.by_index_raw(index).map_err(err)?;
        let name = f.name().to_owned();
        safe_name(&name)?;
        require(names.insert(name.clone()), "duplicate ZIP member")?;
        if let Some(mode) = f.unix_mode() {
            require(
                mode & 0o170000 == 0 || mode & 0o170000 == 0o100000,
                "nonregular ZIP member",
            )?;
        }
        require(f.size() <= CAP, "member cap")?;
        total = total.checked_add(f.size()).ok_or("total overflow")?;
        require(total <= 1024 * 1024 * 1024, "total cap")?;
        out.push((name, f.size()));
    }
    require(
        out.len() == expected && !out.is_empty(),
        "no visited entries",
    )?;
    out.sort();
    Ok(out)
}
fn compare_stream(archive: &Path, name: &str, file: &Path, size: u64) -> Result<String> {
    safe_name(name)?;
    require(
        fs::symlink_metadata(file)
            .map_err(err)?
            .file_type()
            .is_file(),
        "missing/nonregular member",
    )?;
    require(
        fs::metadata(file).map_err(err)?.len() == size,
        "member length mismatch",
    )?;
    let mut local = File::open(file).map_err(err)?;
    let mut child = Command::new("/usr/bin/tar")
        .args(["-xOf"])
        .arg(archive)
        .arg("--")
        .arg(name)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(err)?;
    let mut pipe = child.stdout.take().ok_or("stdout pipe")?;
    let worker = std::thread::spawn(move || -> Result<String> {
        let mut a = [0u8; 65536];
        let mut b = [0u8; 65536];
        let mut total = 0u64;
        let mut h = Sha256::new();
        loop {
            let n = pipe.read(&mut a).map_err(err)?;
            if n == 0 {
                break;
            }
            total = total.checked_add(n as u64).ok_or("stream overflow")?;
            require(total <= size, "archive stream exceeds declared size")?;
            local.read_exact(&mut b[..n]).map_err(err)?;
            require(a[..n] == b[..n], "direct member bytes differ")?;
            h.update(&a[..n]);
        }
        require(total == size, "archive stream truncated")?;
        require(
            local.read(&mut b[..1]).map_err(err)? == 0,
            "local file grew",
        )?;
        Ok(format!("{:x}", h.finalize()))
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let exit = loop {
        match child.try_wait().map_err(err)? {
            Some(status) => break Ok(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("member extractor deadline".to_owned());
            }
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    };
    let result = worker.join().map_err(|_| "reader panicked".to_string())?;
    require(exit?.success(), "libarchive extraction failed")?;
    result
}
fn compare_archive(archive: &Path, remote: &Path, expected: usize) -> Result<Value> {
    let entries = entries(archive, expected)?;
    let actual = inventory(remote)?;
    require(
        entries
            .iter()
            .map(|(n, _)| n.clone())
            .collect::<BTreeSet<_>>()
            == actual,
        "remote inventory differs from actual ZIP",
    )?;
    let mut compared = Vec::new();
    let mut total = 0u64;
    for (name, size) in entries {
        let sha = compare_stream(archive, &name, &remote.join(&name), size)?;
        total = total.checked_add(size).ok_or("sum overflow")?;
        compared.push(json!({"name":name,"bytes":size,"sha256":sha}));
    }
    require(
        compared.len() == expected && !compared.is_empty(),
        "visited member count",
    )?;
    Ok(json!({"members":compared,"visited_members":expected,"uncompressed_bytes_compared":total}))
}
fn old_manifest_gate(path: &Path, expected: usize) -> Result<()> {
    let t = text(path)?;
    require(
        !t.is_empty() && t.lines().count() == expected,
        "old manifest is empty or has wrong count",
    )
}
fn review(root: &Path) -> Result<Value> {
    let correction = root.join("qualification/artifact-member-custody-correction");
    let supplied = text(&correction.join("input-directories.txt"))?;
    let mut expected_dirs = Vec::new();
    for i in 0..5 {
        expected_dirs.push(root.join(format!("campaign/compressed_localized_32/shard-{i}")));
    }
    expected_dirs.push(root.join("campaign/cold_random/shard-0-attempt-1-preflight-failure"));
    for i in 0..5 {
        expected_dirs.push(root.join(format!("campaign/cold_random/shard-{i}")));
    }
    require(
        supplied.lines().map(PathBuf::from).collect::<Vec<_>>() == expected_dirs,
        "mapping differs from fixed eleven paths",
    )?;
    let mut reports = Vec::new();
    let mut all_members = 0usize;
    for dir in expected_dirs {
        let expected = if dir.to_string_lossy().contains("preflight-failure") {
            53
        } else {
            56
        };
        let custody = json_file(&dir.join("evidence-artifact-custody.json"))?;
        let api = json_file(&dir.join("evidence-artifacts-api.json"))?;
        let artifacts = api["artifacts"].as_array().ok_or("API artifacts array")?;
        let matched: Vec<_> = artifacts
            .iter()
            .filter(|a| a["id"] == custody["artifact"]["id"])
            .collect();
        require(matched.len() == 1, "API artifact identity not unique")?;
        let artifact = matched[0];
        require(
            artifact == &custody["artifact"],
            "custody artifact differs from retained API",
        )?;
        require(
            artifact["workflow_run"]["head_sha"] == SOURCE
                && artifact["workflow_run"]["repository_id"] == 1433848227u64,
            "source/repository mismatch",
        )?;
        let archive = dir.join("evidence-artifact.zip");
        let (bytes, sha) = hash_file(&archive, CAP)?;
        require(
            artifact["size_in_bytes"] == bytes && artifact["digest"] == format!("sha256:{sha}"),
            "retained API ZIP digest/size mismatch",
        )?;
        require(
            custody["local_archive"]["bytes"] == bytes && custody["local_archive"]["sha256"] == sha,
            "old custody archive mismatch",
        )?;
        let manifest = dir.join("evidence-remote-files.sha256");
        let (old_bytes, old_sha) = hash_file(&manifest, 1024 * 1024)?;
        require(
            old_bytes == 0 && old_manifest_gate(&manifest, expected).is_err(),
            "expected original empty manifest no longer retained",
        )?;
        let result = compare_archive(&archive, &dir.join("remote"), expected)?;
        let mut manifest_new = String::new();
        for m in result["members"].as_array().ok_or("members")? {
            manifest_new.push_str(&format!(
                "{}  {}\n",
                m["sha256"].as_str().ok_or("sha")?,
                m["name"].as_str().ok_or("name")?
            ));
        }
        let new_sha = format!("{:x}", Sha256::digest(manifest_new.as_bytes()));
        all_members += expected;
        reports.push(json!({"directory":dir,"artifact_id":artifact["id"],"run_id":artifact["workflow_run"]["id"],"archive_bytes":bytes,"archive_sha256":sha,"api_file_sha256":hash_file(&dir.join("evidence-artifacts-api.json"),1024*1024)?.1,"original_custody_sha256":hash_file(&dir.join("evidence-artifact-custody.json"),1024*1024)?.1,"original_empty_manifest_sha256":old_sha,"old_member_claim_withdrawn":true,"corrected_manifest_sha256":new_sha,"comparison":result}));
    }
    require(
        reports.len() == 11 && all_members == 613,
        "campaign mandatory counts",
    )?;
    let source_path = correction.join("independent/review.rs");
    let executable = std::env::current_exe().map_err(err)?;
    Ok(
        json!({"schema":"spiralcore.independent-member-custody-review.v1","status":"passed","runtime_source":SOURCE,"archives":11,"members_compared":613,"review_source_sha256":hash_file(&source_path,1024*1024)?.1,"review_executable_sha256":hash_file(&executable,CAP)?.1,"method":"Enumerate actual ZIP central-directory entries with Rust zip; independently extract each entry through macOS libarchive/bsdtar and compare the stream byte-for-byte with the existing remote file. Never iterate the historical supplied list.","records":reports,"limitations":["Retained API responses are bound locally; this review performs no new GitHub authentication.","This is archive/file custody recovery, not a new transfer, capture-completeness judgment or performance measurement.","Forty-five earlier populated manifests were censused only, not re-compared by this run."],"original_inputs_modified":false}),
    )
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = if args.len() == 3 && args[1] == "--assert-old" {
        old_manifest_gate(Path::new(&args[2]), 56).map(|_| json!({"old_gate":"passed"}))
    } else if args.len() == 2 {
        review(Path::new(&args[1]))
    } else {
        Err("usage: review ROOT | --assert-old PATH".into())
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).expect("JSON")),
        Err(e) => {
            eprintln!("independent custody: {e}");
            std::process::exit(1)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    fn fixture(label: &str, empty: bool) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "spiralcore-custody-negative-{}-{label}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let remote = root.join("remote");
        fs::create_dir(&remote).unwrap();
        let archive = root.join("fixture.zip");
        let mut z = zip::ZipWriter::new(File::create(&archive).unwrap());
        if !empty {
            z.start_file(
                "a.txt",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            z.write_all(b"expected bytes\0\n").unwrap();
            fs::write(remote.join("a.txt"), b"expected bytes\0\n").unwrap();
        }
        z.finish().unwrap();
        (root, archive, remote)
    }
    #[test]
    fn actual_extraction_exact_bytes_pass() {
        let (r, a, d) = fixture("exact", false);
        assert_eq!(compare_archive(&a, &d, 1).unwrap()["visited_members"], 1);
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn changed_bytes_rejected() {
        let (r, a, d) = fixture("changed", false);
        fs::write(d.join("a.txt"), b"incorrect byte\0\n").unwrap();
        assert!(compare_archive(&a, &d, 1).is_err());
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn missing_member_rejected() {
        let (r, a, d) = fixture("missing", false);
        fs::remove_file(d.join("a.txt")).unwrap();
        assert!(compare_archive(&a, &d, 1).is_err());
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn empty_iteration_rejected() {
        let (r, a, d) = fixture("empty", true);
        assert!(compare_archive(&a, &d, 0).is_err());
        assert!(compare_archive(&a, &d, 1).is_err());
        fs::remove_dir_all(r).unwrap();
    }
    #[test]
    fn supplied_count_cannot_hide_missing_work() {
        let (r, a, d) = fixture("count", false);
        assert!(compare_archive(&a, &d, 2).is_err());
        fs::remove_dir_all(r).unwrap();
    }
}
