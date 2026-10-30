use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Component, Path};
use std::process::{Command, Stdio};
type R<T> = Result<T, String>;
const RECORDS: &str = "8e46471f8aacf74a931b0152ebacf656f7690376";
const SOURCE: &str = "9dce4c57d1f41cef7365b297cd690849afce34e7";
const HASH: &str = "f976473d124d3f5989efa3ce7176176d7eb19d4166bae9567bad4062777aa8eb";
fn e(v: impl std::fmt::Display) -> String {
    v.to_string()
}
fn ck(b: bool, m: &str) -> R<()> {
    if b {
        Ok(())
    } else {
        Err(m.into())
    }
}
fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn bounded(path: &Path) -> R<Vec<u8>> {
    let mut v = Vec::new();
    File::open(path)
        .map_err(e)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut v)
        .map_err(e)?;
    ck(v.len() <= 16 * 1024 * 1024, "metadata cap")?;
    Ok(v)
}
fn s(v: &Value) -> R<&str> {
    v.as_str().ok_or("string field missing".into())
}
fn u(v: &Value) -> R<u64> {
    v.as_u64().ok_or("unsigned field missing".into())
}
fn arr(v: &Value) -> R<&Vec<Value>> {
    v.as_array().ok_or("array missing".into())
}
fn safe(p: &str) -> R<()> {
    ck(
        !p.is_empty()
            && p.len() <= 512
            && p.bytes()
                .all(|x| x.is_ascii_alphanumeric() || b"_./-".contains(&x))
            && Path::new(p)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            && !p.contains("//")
            && !p.ends_with('/'),
        "unsafe tar name",
    )
}
fn oct(b: &[u8]) -> R<u64> {
    let t = std::str::from_utf8(b)
        .map_err(e)?
        .trim_matches(|c| c == '\0' || c == ' ');
    ck(
        !t.is_empty() && t.bytes().all(|x| (b'0'..=b'7').contains(&x)),
        "invalid octal",
    )?;
    u64::from_str_radix(t, 8).map_err(e)
}
fn zstr(b: &[u8]) -> R<String> {
    let n = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    ck(b[n..].iter().all(|&x| x == 0), "nonzero string suffix")?;
    Ok(std::str::from_utf8(&b[..n]).map_err(e)?.to_owned())
}
struct Stream<T> {
    r: T,
    h: Sha256,
    n: u64,
}
impl<T: Read> Stream<T> {
    fn read_exact(&mut self, b: &mut [u8]) -> R<()> {
        self.r.read_exact(b).map_err(e)?;
        self.h.update(&*b);
        self.n = self
            .n
            .checked_add(b.len() as u64)
            .ok_or("offset overflow")?;
        ck(self.n <= 2 * 1024 * 1024 * 1024, "archive cap")
    }
}
fn tar<T: Read>(
    r: T,
    expected: &BTreeMap<String, (u64, String)>,
) -> R<(Value, BTreeMap<String, Vec<u8>>)> {
    ck(
        !expected.is_empty() && expected.len() <= 68,
        "positive bounded expected entries",
    )?;
    let mut input = Stream {
        r,
        h: Sha256::new(),
        n: 0,
    };
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let mut controls = BTreeMap::new();
    loop {
        let start = input.n;
        let mut h = [0u8; 512];
        input.read_exact(&mut h)?;
        if h.iter().all(|&b| b == 0) {
            let mut second = [0u8; 512];
            input.read_exact(&mut second)?;
            ck(second.iter().all(|&b| b == 0), "missing second end block")?;
            break;
        }
        ck(
            &h[257..263] == b"ustar\0" && &h[263..265] == b"00",
            "non-ustar header",
        )?;
        let sum: u64 = h
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                if (148..156).contains(&i) {
                    32
                } else {
                    x as u64
                }
            })
            .sum();
        ck(sum == oct(&h[148..156])?, "header checksum mismatch")?;
        ck(h[156] == b'0' || h[156] == 0, "not a regular file")?;
        ck(
            h[157..257].iter().all(|&b| b == 0),
            "unexpected link target",
        )?;
        let leaf = zstr(&h[..100])?;
        let prefix = zstr(&h[345..500])?;
        let name = if prefix.is_empty() {
            leaf
        } else {
            format!("{prefix}/{leaf}")
        };
        safe(&name)?;
        ck(seen.insert(name.clone()), "duplicate member")?;
        let want = expected
            .get(&name)
            .ok_or(format!("unexpected member {name}"))?;
        let size = oct(&h[124..136])?;
        ck(
            size == want.0 && size <= 128 * 1024 * 1024,
            "member size mismatch/cap",
        )?;
        let keep = matches!(
            name.as_str(),
            "inventory.json" | "MEMBERS.txt" | "SHA256SUMS" | "README.md"
        );
        if keep {
            ck(size <= 16 * 1024 * 1024, "control member cap")?;
        }
        let mut retained = Vec::new();
        let mut digest = Sha256::new();
        let mut left = size;
        let mut buf = [0u8; 65536];
        while left > 0 {
            let n = (left as usize).min(buf.len());
            input.read_exact(&mut buf[..n])?;
            digest.update(&buf[..n]);
            if keep {
                retained.extend_from_slice(&buf[..n]);
            }
            left -= n as u64;
        }
        let got = format!("{:x}", digest.finalize());
        ck(got == want.1, "member digest mismatch")?;
        let pad = ((512 - size % 512) % 512) as usize;
        input.read_exact(&mut buf[..pad])?;
        ck(buf[..pad].iter().all(|&b| b == 0), "nonzero member padding")?;
        if keep {
            controls.insert(name.clone(), retained);
        }
        rows.push(json!({"name":name,"bytes":size,"sha256":got,"header_offset":start,"regular_file":true}));
        ck(rows.len() <= expected.len(), "too many members")?;
    }
    let mut tail = [0u8; 8192];
    let mut trailing = 0u64;
    loop {
        let n = input.r.read(&mut tail).map_err(e)?;
        if n == 0 {
            break;
        }
        ck(
            tail[..n].iter().all(|&x| x == 0),
            "nonzero bytes after tar end",
        )?;
        input.h.update(&tail[..n]);
        input.n = input.n.checked_add(n as u64).ok_or("archive overflow")?;
        trailing += n as u64;
        ck(trailing <= 10240, "excess tar trailing padding")?;
    }
    ck(
        seen == expected.keys().cloned().collect(),
        "missing member or incomplete iteration",
    )?;
    Ok((
        json!({"members":rows,"member_count":seen.len(),"archive_bytes":input.n,"archive_sha256":format!("{:x}",input.h.finalize()),"trailing_zero_bytes_after_two_end_blocks":trailing}),
        controls,
    ))
}
fn git(repo: &Path, args: &[&str]) -> R<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(e)?;
    ck(out.status.success(), "git failed")?;
    ck(out.stdout.len() <= 16 * 1024 * 1024, "git metadata cap")?;
    Ok(out.stdout)
}
fn show(repo: &Path, path: &str) -> R<Value> {
    safe(path)?;
    serde_json::from_slice(&git(repo, &["show", &format!("{RECORDS}:{path}")])?).map_err(e)
}
fn git_archive(repo: &Path, rev: &str, prefix: &str, roots: &[&str]) -> R<(u64, String)> {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .args([
            "archive",
            "--format=tar",
            &format!("--prefix={prefix}"),
            rev,
            "--",
        ])
        .args(roots)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut c = cmd.spawn().map_err(e)?;
    let mut r = c.stdout.take().ok_or("git archive pipe")?;
    let mut h = Sha256::new();
    let mut n = 0u64;
    let mut b = [0u8; 65536];
    loop {
        let k = r.read(&mut b).map_err(e)?;
        if k == 0 {
            break;
        }
        n += k as u64;
        if n > 128 * 1024 * 1024 {
            let _ = c.kill();
            let _ = c.wait();
            return Err("git archive cap".into());
        }
        h.update(&b[..k]);
    }
    ck(c.wait().map_err(e)?.success(), "git archive failed")?;
    Ok((n, format!("{:x}", h.finalize())))
}
fn review(root: &Path, repo: &Path) -> R<Value> {
    let build = root.join("review-bundle-build");
    let manifest_bytes = bounded(&build.join("members.json"))?;
    ck(
        sha(&manifest_bytes) == "c74f6826b79cc38943c34267bb2f939cfa5ab1e8e2f1fdaa948c49e6d46011f2",
        "manifest pin",
    )?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(e)?;
    ck(
        manifest["schema"] == "spiralcore.review-bundle-outer-members.v1",
        "manifest schema",
    )?;
    let mut expected = BTreeMap::new();
    for m in arr(&manifest["members"])? {
        let name = s(&m["name"])?;
        safe(name)?;
        let digest = s(&m["sha256"])?;
        ck(
            digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
            "digest shape",
        )?;
        ck(
            expected
                .insert(name.to_owned(), (u(&m["bytes"])?, digest.to_owned()))
                .is_none(),
            "duplicate expected name",
        )?;
    }
    ck(expected.len() == 68, "expected68")?;
    let archive = root.join("broad-wan-20261029-review.tar");
    ck(
        fs::symlink_metadata(&archive)
            .map_err(e)?
            .file_type()
            .is_file(),
        "archive not regular",
    )?;
    ck(
        fs::metadata(&archive).map_err(e)?.len() == 1224990720,
        "archive length",
    )?;
    let (observed, controls) = tar(
        BufReader::with_capacity(65536, File::open(&archive).map_err(e)?),
        &expected,
    )?;
    ck(
        observed["archive_bytes"] == 1224990720u64 && observed["archive_sha256"] == HASH,
        "archive SHA/length mismatch",
    )?;
    for name in ["MEMBERS.txt", "README.md", "SHA256SUMS", "inventory.json"] {
        ck(
            controls.get(name).ok_or("control absent")?
                == &bounded(&root.join("review-bundle-staging").join(name))?,
            "control differs from staging",
        )?;
    }
    let names: Vec<_> = std::str::from_utf8(&controls["MEMBERS.txt"])
        .map_err(e)?
        .lines()
        .collect();
    ck(
        names == expected.keys().map(String::as_str).collect::<Vec<_>>(),
        "MEMBERS must list exact68",
    )?;
    let mut checks = BTreeMap::new();
    for line in std::str::from_utf8(&controls["SHA256SUMS"])
        .map_err(e)?
        .lines()
    {
        let (hash, name) = line.split_once("  ").ok_or("checksum format")?;
        ck(
            name != "SHA256SUMS" && checks.insert(name, hash).is_none(),
            "checksum duplicate/self",
        )?;
        ck(
            expected.get(name).is_some_and(|x| x.1 == hash),
            "checksum disagreement",
        )?;
    }
    ck(checks.len() == 67, "checksum count67")?;
    let inv: Value = serde_json::from_slice(&controls["inventory.json"]).map_err(e)?;
    ck(
        inv["schema"] == "spiralcore.broad-wan-review-bundle-inventory.v1"
            && inv["records_revision"] == RECORDS
            && inv["measured_revision"] == SOURCE
            && inv["expected_outer_members"] == 68,
        "inventory identity",
    )?;
    let cases = [
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
    ];
    let mut pairs = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut roles = BTreeMap::<String, usize>::new();
    let mut custody = Vec::new();
    let mut cached = BTreeMap::<String, Value>::new();
    for a in arr(&inv["artifacts"])? {
        let name = s(&a["name"])?;
        let want = expected.get(name).ok_or("artifact missing from outer")?;
        ck(
            want.0 == u(&a["bytes"])? && want.1 == s(&a["sha256"])?,
            "artifact digest/length not outer",
        )?;
        ck(
            ids.insert(u(&a["artifact_id"])?),
            "duplicate artifact identity",
        )?;
        let role = s(&a["role"])?;
        *roles.entry(role.to_owned()).or_default() += 1;
        if role == "measured_artifact" {
            let case = s(&a["case"])?;
            let shard = u(&a["shard"])?;
            ck(
                cases.contains(&case)
                    && shard < 5
                    && name == format!("artifacts/{case}/shard-{shard}.zip")
                    && pairs.insert((case.to_owned(), shard)),
                "case/shard coverage",
            )?;
            let path = s(&a["custody_index"])?;
            ck(path==format!("docs/network/evidence/2026-10-29-broad-wan/cases/{case}/hosted-artifact-custody.json"),"custody path")?;
            if !cached.contains_key(path) {
                cached.insert(path.to_owned(), show(repo, path)?);
            }
            let idx = &cached[path];
            ck(
                idx["source"] == SOURCE && idx["suite"] == format!("broad16-{case}"),
                "custody source/suite",
            )?;
            let matched: Vec<_> = arr(&idx["artifacts"])?
                .iter()
                .filter(|x| x["shard"] == shard)
                .collect();
            ck(matched.len() == 1, "custody shard uniqueness")?;
            for k in ["run_id", "artifact_id", "bytes", "sha256"] {
                ck(matched[0][k] == a[k], "custody field mismatch")?;
            }
        } else if role == "setup_failure_artifact" {
            ck(
                a["run_id"] == 40984573497u64
                    && a["artifact_id"] == 12946655973u64
                    && a["measured_operations"] == 0,
                "failure identity",
            )?;
            let idx = show(repo, s(&a["custody_index"])?)?;
            ck(
                idx["artifact"]["id"] == a["artifact_id"]
                    && idx["artifact"]["workflow_run"]["id"] == a["run_id"]
                    && idx["local_archive"]["bytes"] == a["bytes"]
                    && idx["local_archive"]["sha256"] == a["sha256"],
                "failure custody",
            )?;
        } else if role == "source_qualified_producer" {
            let idx=show(repo,"docs/network/evidence/2026-10-29-broad-wan/qualification/linux-producer-2/independent-runtime-custody-review.json")?;
            ck(
                idx["status"] == "passed"
                    && idx["source"] == SOURCE
                    && idx["producer"]["run_id"] == a["run_id"]
                    && idx["producer"]["artifact_id"] == a["artifact_id"]
                    && idx["producer"]["archive_bytes"] == a["bytes"]
                    && idx["producer"]["artifact_digest"] == format!("sha256:{}", s(&a["sha256"])?)
                    && idx["manifest"] == a["manifest"],
                "producer custody",
            )?;
        } else {
            return Err("unknown artifact role".into());
        }
        custody.push(json!({"name":name,"artifact_id":a["artifact_id"],"run_id":a["run_id"],"reference_matches":true}));
    }
    ck(
        pairs.len() == 60
            && roles.get("measured_artifact") == Some(&60)
            && roles.get("setup_failure_artifact") == Some(&1)
            && roles.get("source_qualified_producer") == Some(&1)
            && roles.len() == 3,
        "artifact role counts",
    )?;
    let mut inner = Vec::new();
    ck(arr(&inv["inner_archives"])?.len() == 2, "inner count")?;
    for a in arr(&inv["inner_archives"])? {
        let name = s(&a["name"])?;
        let (rev, prefix, roots, count, bytes) = match name {
            "compact-records.tar" => (
                RECORDS,
                "compact-records/",
                vec!["docs/network", "docs/plans", "docs/specs"],
                5563,
                60997166u64,
            ),
            "measured-source.tar" => (
                SOURCE,
                "measured-source/",
                vec![
                    "README.md",
                    "PROVENANCE.md",
                    "crates/spiralcore-transfer",
                    "crates/uor-r4-object-store",
                    "tools",
                    ".github",
                ],
                67,
                943050u64,
            ),
            _ => return Err("inner name".into()),
        };
        ck(
            a["source_revision"] == rev
                && a["prefix"] == prefix
                && a["selected_roots"] == json!(roots),
            "inner selection",
        )?;
        let mut args = vec!["ls-tree", "-rlz", rev, "--"];
        args.extend(&roots);
        let tree = git(repo, &args)?;
        let mut actual = Vec::new();
        for item in tree.split(|&x| x == 0).filter(|x| !x.is_empty()) {
            let t = std::str::from_utf8(item).map_err(e)?;
            let (meta, path) = t.split_once('\t').ok_or("tree format")?;
            let fields: Vec<_> = meta.split_whitespace().collect();
            ck(
                fields.len() == 4
                    && fields[1] == "blob"
                    && ["100644", "100755"].contains(&fields[0]),
                "tree nonregular",
            )?;
            actual.push(json!({"mode":fields[0],"type":fields[1],"blob":fields[2],"bytes":fields[3].parse::<u64>().map_err(e)?,"path":path}));
        }
        ck(
            actual == *arr(&a["files"])?
                && actual.len() == count
                && actual
                    .iter()
                    .map(|x| x["bytes"].as_u64().unwrap())
                    .sum::<u64>()
                    == bytes,
            "inner Git inventory",
        )?;
        let (archive_bytes, archive_sha) = git_archive(repo, rev, prefix, &roots)?;
        let want = expected.get(name).ok_or("inner missing outer")?;
        ck(
            want.0 == archive_bytes
                && want.1 == archive_sha
                && a["bytes"] == archive_bytes
                && a["sha256"] == archive_sha,
            "regenerated Git archive not byte-identical",
        )?;
        inner.push(json!({"name":name,"source":rev,"files":count,"source_bytes":bytes,"archive_bytes":archive_bytes,"sha256":archive_sha,"regenerated_git_stream_identical":true}));
    }
    let report = git(
        repo,
        &[
            "show",
            &format!("{RECORDS}:docs/network/broad-wan-results-2026-10-29.md"),
        ],
    )?;
    ck(
        sha(&report) == s(&inv["report"]["sha256"])?,
        "report digest",
    )?;
    let author_bytes = bounded(&build.join("stream-verification.json"))?;
    let author: Value = serde_json::from_slice(&author_bytes).map_err(e)?;
    ck(
        author["status"] == "passed"
            && author["archive_bytes"] == observed["archive_bytes"]
            && author["archive_sha256"] == observed["archive_sha256"],
        "author outer mismatch",
    )?;
    for k in [
        "expected_members",
        "listed_members",
        "regular_file_members",
        "streamed_members",
    ] {
        ck(author[k] == 68, "author count")?;
    }
    Ok(
        json!({"schema":"spiralcore.portable-bundle-independent-review.v1","status":"passed","records_revision":RECORDS,"measured_revision":SOURCE,"archive":archive,"archive_bytes":observed["archive_bytes"],"archive_sha256":observed["archive_sha256"],"member_count":68,"actual_stream":observed,"artifact_custody_references":custody,"inner_archives":inner,"report_sha256":sha(&report),"manifest_sha256":sha(&manifest_bytes),"inventory_sha256":sha(&controls["inventory.json"]),"author_verification_sha256":sha(&author_bytes),"method":"Independent bounded single-pass ustar parser verifies header checksums, regular-only unique safe paths, sizes, every payload SHA256, zero padding, exact68 names,67 checksums and complete archive SHA256. Two regenerated Git archive streams match included inner archive bytes; no disk extraction.","limitations":["Inner ZIP member checks were not repeated; their original ZIP bytes and retained custody references are bound.","No new external authentication, transfer, benchmark, packet replay or original-cache verification occurred.","Authenticated archive identity is distinct from the historical member-check claims superseded by the retained correction.","Portable source does not vendor the compiler, dependencies, operating environment or deleted receiver caches."],"original_inputs_modified":false,"expanded_extraction_created":false}),
    )
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let result = if a.len() == 3 {
        review(Path::new(&a[1]), Path::new(&a[2]))
    } else {
        Err("usage ROOT REPO".into())
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(err) => {
            eprintln!("bundle independent: {err}");
            std::process::exit(1)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Vec<u8>, BTreeMap<String, (u64, String)>) {
        let mut h = [0u8; 512];
        h[0] = b'a';
        h[124..136].copy_from_slice(b"00000000003\0");
        h[148..156].fill(b' ');
        h[156] = b'0';
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u64 = h.iter().map(|&x| x as u64).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        let mut v = h.to_vec();
        v.extend(b"abc");
        v.resize(2048, 0);
        (v, BTreeMap::from([("a".into(), (3, sha(b"abc")))]))
    }
    #[test]
    fn exact() {
        let (v, m) = fixture();
        assert_eq!(tar(&v[..], &m).unwrap().0["member_count"], 1);
    }
    #[test]
    fn corrupted_payload() {
        let (mut v, m) = fixture();
        v[512] ^= 1;
        assert!(tar(&v[..], &m).is_err());
    }
    #[test]
    fn truncated() {
        let (v, m) = fixture();
        assert!(tar(&v[..1536], &m).is_err());
    }
    #[test]
    fn missing_member() {
        let (v, mut m) = fixture();
        m.insert("b".into(), (0, sha(b"")));
        assert!(tar(&v[..], &m).is_err());
    }
    #[test]
    fn duplicate_member() {
        let (v, m) = fixture();
        let mut d = v[..1024].to_vec();
        d.extend_from_slice(&v);
        assert!(tar(&d[..], &m).is_err());
    }
    #[test]
    fn zero_work_rejected() {
        assert!(tar(&[0u8; 1024][..], &BTreeMap::new()).is_err());
    }
}
