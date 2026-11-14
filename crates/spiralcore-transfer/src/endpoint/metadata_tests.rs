//! Metadata equivalence and a bounded same-executable Windows comparison.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Fixture(pub(super) PathBuf);
impl Fixture {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "spiral-metadata-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        storage::private_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
// Exact old path, retained only as the experimental control.
fn legacy_read_meta(path: &Path) -> Result<Publication> {
    match fs::symlink_metadata(path) {
        Ok(_) => private_file(path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::Unknown),
        Err(_) => return Err(Error::Storage),
    }
    let file = storage::open_read(path)?;
    read_meta_file(Locator::Path(path.to_owned()), &file)
}
fn signature(p: &Publication) -> (u64, Id, Option<Id>) {
    (p.len, p.object, p.manifest)
}
fn write_record(path: &Path, bytes: &[u8]) {
    let mut file = storage::create_new_file(path).unwrap();
    file.write_all(b"SCPUB001").unwrap();
    file.write_all(&(bytes.len() as u64).to_le_bytes()).unwrap();
    file.write_all(&object_id(bytes)).unwrap();
    file.write_all(&[0]).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn both_refuse(path: &Path, expected: Error) {
    assert_eq!(legacy_read_meta(path).err(), Some(expected));
    assert_eq!(read_meta(path).err(), Some(expected));
}
#[test]
fn metadata_paths_preserve_header_length_missing_and_exact_body_checks() {
    let fixture = Fixture::new();
    assert_legacy_record_bytes(&fixture);
    let path = fixture.0.join("payload.object");
    let bytes = b"\0exact metadata\0trailing zeros\0\0";
    write_record(&path, bytes);
    let old = legacy_read_meta(&path).unwrap();
    let new = read_meta(&path).unwrap();
    assert_eq!(signature(&old), signature(&new));
    assert_eq!(read_publication(&old).unwrap(), bytes);
    assert_eq!(read_publication(&new).unwrap(), bytes);
    both_refuse(&fixture.0.join("missing.object"), Error::Unknown);
    both_refuse(&fixture.0, Error::Storage);
    let original = fs::read(&path).unwrap();
    let mut corrupt = original.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&path, &corrupt).unwrap();
    // Metadata never grants verified-byte status: subsequent actual readback still hashes.
    assert_eq!(
        read_publication(&read_meta(&path).unwrap()),
        Err(Error::Integrity)
    );
    fs::write(&path, &original[..48]).unwrap();
    both_refuse(&path, Error::Integrity);
    let mut extra = original.clone();
    extra.push(0);
    fs::write(&path, &extra).unwrap();
    both_refuse(&path, Error::Integrity);
    let mut flag = original.clone();
    flag[48] = 2;
    fs::write(&path, &flag).unwrap();
    both_refuse(&path, Error::Integrity);
    fs::write(&path, &original).unwrap();
    assert_eq!(read_publication(&read_meta(&path).unwrap()).unwrap(), bytes);
}

fn assert_legacy_record_bytes(fixture: &Fixture) {
    const PREFIX: &[u8; 48] = b"SCPUB001\x05\0\0\0\0\0\0\0\
        \x83\x52\x42\x8b\xef\x1b\x1f\x9f\xc5\xf9\x4c\xfb\xf4\x9d\x60\x67\
        \xfa\xb8\xc0\xb3\x15\x0c\x86\x06\x2a\xe9\x0d\x1c\x4b\x36\xf1\x1b";
    const BODY: &[u8] = b"\0\xffA\0\0";
    let offer = Offer::cold(BODY, Mode::Raw);
    for (name, manifest, suffix) in [
        ("plain.object", None, &b"\0"[..]),
        (
            "manifest.object",
            Some(*b"abcdefghijklmnopqrstuvwxyz012345"),
            &b"\x01abcdefghijklmnopqrstuvwxyz012345"[..],
        ),
    ] {
        let path = fixture.0.join(name);
        let mut file = storage::create_new_file(&path).unwrap();
        super::write_record(&mut file, &offer, manifest, BODY).unwrap();
        drop(file);
        assert_eq!(
            fs::read(path).unwrap(),
            [PREFIX.as_slice(), suffix, BODY].concat()
        );
    }
}

#[test]
fn metadata_paths_reject_hardlinks_without_changing_the_original_bytes() {
    let fixture = Fixture::new();
    let path = fixture.0.join("payload.object");
    write_record(&path, b"original\0bytes");
    let original = fs::read(&path).unwrap();
    let alias = fixture.0.join("alias");
    fs::hard_link(&path, &alias).unwrap();
    both_refuse(&path, Error::Storage);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(&alias).unwrap(), original);
    fs::remove_file(alias).unwrap();
    assert_eq!(
        read_publication(&read_meta(&path).unwrap()).unwrap(),
        b"original\0bytes"
    );
}

#[cfg(windows)]
mod windows {
    use super::*;
    use cpu_time::{ProcessTime, ThreadTime};
    use std::{process::Command, time::Instant};
    struct Sample {
        wall_ns: u128,
        process_cpu_ns: u128,
        thread_cpu_ns: u128,
    }
    fn measured(action: impl FnOnce()) -> Sample {
        let process = ProcessTime::try_now().expect("actual process CPU clock");
        let thread = ThreadTime::try_now().expect("actual thread CPU clock");
        let began = Instant::now();
        action();
        let wall = began.elapsed();
        Sample {
            wall_ns: wall.as_nanos(),
            process_cpu_ns: process.try_elapsed().unwrap().as_nanos(),
            thread_cpu_ns: thread.try_elapsed().unwrap().as_nanos(),
        }
    }
    fn sample(s: &Sample) -> serde_json::Value {
        serde_json::json!({"wall_ns":s.wall_ns,"process_cpu_ns":s.process_cpu_ns,"thread_cpu_ns":s.thread_cpu_ns})
    }
    fn powershell(path: &Path, sddl: &str, script: &str) -> String {
        let program =
            PathBuf::from(std::env::var_os("SystemRoot").expect("native Windows system root"))
                .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        // PS7 -> Cargo -> Windows PowerShell otherwise inherits PS7 module paths.
        // Let this child reconstruct its own defaults without mutating the parent.
        let output = Command::new(program)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .env_remove("PSModulePath")
            .env("SC_METADATA_TEST_FILE", path)
            .env("SC_METADATA_TEST_SDDL", sddl)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "owned fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
    const ACL_READ: &str =
        "$ErrorActionPreference='Stop'; (Get-Acl -LiteralPath $env:SC_METADATA_TEST_FILE).Sddl";
    #[derive(serde::Deserialize)]
    struct AclSnapshot {
        sddl: String,
        canonical: String,
        control: u32,
    }
    fn acl_snapshot(path: &Path) -> AclSnapshot {
        // Use binary input: SDDL omits some descriptor control flags. Only the
        // OS-managed DACL AutoInherited bit is ignored for restoration equality.
        let output = powershell(
            path,
            "",
            r#"
$ErrorActionPreference='Stop'
$acl=Get-Acl -LiteralPath $env:SC_METADATA_TEST_FILE
[byte[]]$binary=$acl.GetSecurityDescriptorBinaryForm()
$descriptor=[System.Security.AccessControl.RawSecurityDescriptor]::new($binary,0)
$flags=[int]$descriptor.ControlFlags
$required=([int][System.Security.AccessControl.ControlFlags]::DiscretionaryAclPresent) -bor ([int][System.Security.AccessControl.ControlFlags]::DiscretionaryAclProtected) -bor ([int][System.Security.AccessControl.ControlFlags]::SelfRelative)
if (($flags -band $required) -ne $required) { throw 'Expected a present protected self-relative DACL' }
$ignore=[int][System.Security.AccessControl.ControlFlags]::DiscretionaryAclAutoInherited
$descriptor.SetFlags([System.Security.AccessControl.ControlFlags]($flags -band (-bnot $ignore)))
[byte[]]$normalized=[byte[]]::new($descriptor.BinaryLength)
$descriptor.GetBinaryForm($normalized,0)
@{sddl=$acl.Sddl;canonical=[Convert]::ToBase64String($normalized);control=$flags} | ConvertTo-Json -Compress
"#,
        );
        serde_json::from_str(&output).expect("owned descriptor snapshot")
    }
    fn verify_all(records: &[(PathBuf, Vec<u8>)]) {
        for (path, bytes) in records {
            for read in [
                legacy_read_meta as fn(&Path) -> Result<Publication>,
                read_meta,
            ] {
                let publication = read(path).unwrap();
                assert_eq!(publication.len, bytes.len() as u64);
                assert_eq!(publication.object, object_id(bytes));
                assert_eq!(publication.manifest, None);
                assert!(
                    read_publication(&publication).unwrap() == *bytes,
                    "exact original body"
                );
            }
        }
    }
    #[test]
    fn single_checked_open_matches_guards_and_records_paired_cost() {
        const FILES: usize = 64;
        const PAYLOAD: usize = 16 * 1024;
        let fixture = Fixture::new();
        let namespace = fixture.0.join(hex(&[0xc4; 32]));
        storage::private_dir(&namespace).unwrap();
        let mut records = Vec::with_capacity(FILES);
        let setup = measured(|| {
            for index in 0..FILES {
                let mut bytes = vec![0x61; PAYLOAD];
                bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
                bytes[PAYLOAD - 1] = 0;
                let path = namespace.join(format!("{}.object", hex(&object_id(&bytes))));
                write_record(&path, &bytes);
                records.push((path, bytes));
            }
        });
        let serialized: u64 = records
            .iter()
            .map(|(path, _)| fs::metadata(path).unwrap().len())
            .sum();
        assert_eq!(serialized, FILES as u64 * (PAYLOAD as u64 + 49));
        assert!(serialized < 2 * 1024 * 1024);
        let common_warm = measured(|| verify_all(&records));
        eprintln!(
            "METADATA_OPEN_SETUP {}",
            serde_json::json!({"files":FILES,"payload_bytes_each":PAYLOAD,"serialized_bytes":serialized,"cpu_domains_overlap":true,"scope":"synthetic native metadata comparison, not network or MVM","record_create_sync":sample(&setup),"common_warm_verify":sample(&common_warm),"cold_cache_claim":false,"per_arm_cache_state":"common primed metadata and exact bodies"})
        );
        let run = |read: fn(&Path) -> Result<Publication>| {
            measured(|| {
                for (path, bytes) in &records {
                    let p = read(path).unwrap();
                    std::hint::black_box(signature(&p));
                    assert_eq!(p.len, bytes.len() as u64);
                }
            })
        };
        for trial in 0..6 {
            let (old, new) = if trial % 2 == 0 {
                let old = run(legacy_read_meta);
                (old, run(read_meta))
            } else {
                let new = run(read_meta);
                (run(legacy_read_meta), new)
            };
            verify_all(&records);
            eprintln!(
                "METADATA_OPEN_PAIR {}",
                serde_json::json!({"trial":trial,"order":if trial%2==0 {"old-new"}else{"new-old"},"files":FILES,"old":sample(&old),"candidate":sample(&new),"exact_readbacks":true,"cpu_domains_overlap":true,"timed_scope":"metadata header reads; full body verification excluded"})
            );
        }
        let path = &records[0].0;
        both_refuse(&namespace.join("missing.object"), Error::Unknown);
        both_refuse(&namespace, Error::Storage);
        let alias = namespace.join("hardlink.alias");
        fs::hard_link(path, &alias).unwrap();
        both_refuse(path, Error::Storage);
        fs::remove_file(alias).unwrap();
        let writer = fs::OpenOptions::new().write(true).open(path).unwrap();
        both_refuse(path, Error::Storage);
        drop(writer);
        let original_descriptor = acl_snapshot(path);
        let original_acl = &original_descriptor.sddl;
        powershell(path,"","$ErrorActionPreference='Stop'; $acl=Get-Acl -LiteralPath $env:SC_METADATA_TEST_FILE; $sid=[System.Security.Principal.SecurityIdentifier]::new('S-1-1-0'); $rule=[System.Security.AccessControl.FileSystemAccessRule]::new($sid,[System.Security.AccessControl.FileSystemRights]::FullControl,[System.Security.AccessControl.AccessControlType]::Allow); $acl.AddAccessRule($rule); Set-Acl -LiteralPath $env:SC_METADATA_TEST_FILE -AclObject $acl");
        let broad = powershell(path, "", ACL_READ);
        assert_ne!(&broad, original_acl);
        both_refuse(path, Error::Storage);
        assert_eq!(
            powershell(path, "", ACL_READ),
            broad,
            "reader must not repair permissions"
        );
        powershell(path,original_acl,"$ErrorActionPreference='Stop'; $acl=Get-Acl -LiteralPath $env:SC_METADATA_TEST_FILE; $acl.SetSecurityDescriptorSddlForm($env:SC_METADATA_TEST_SDDL,[System.Security.AccessControl.AccessControlSections]::Access); Set-Acl -LiteralPath $env:SC_METADATA_TEST_FILE -AclObject $acl");
        let restored_descriptor = acl_snapshot(path);
        assert_eq!(
            restored_descriptor.control & !0x0400,
            original_descriptor.control & !0x0400,
            "every control bit except SE_DACL_AUTO_INHERITED must match"
        );
        assert_eq!(
            restored_descriptor.canonical, original_descriptor.canonical,
            "restoration preserves owner, group, ACL bytes and every other control bit"
        );
        verify_all(&records[..1]);
        powershell(&namespace,"","$ErrorActionPreference='Stop'; $parent=Split-Path -Parent $env:SC_METADATA_TEST_FILE; New-Item -ItemType Junction -Path (Join-Path $parent 'metadata-junction') -Target $env:SC_METADATA_TEST_FILE | Out-Null");
        let junction = fixture.0.join("metadata-junction");
        let through = junction.join(path.file_name().unwrap());
        both_refuse(&through, Error::Storage);
        fs::remove_dir(&junction).unwrap();
        verify_all(&records);
        assert_eq!(fs::read_dir(&namespace).unwrap().count(), FILES);
        eprintln!("METADATA_OPEN_GUARDS missing=true directory=true hardlink=true conflicting_writer=true broad_acl=true reparse_ancestor=true original_bytes_exact=true serialized_bytes={serialized}");
        let root = fixture.0.clone();
        drop(records);
        drop(fixture);
        assert!(
            !root.exists(),
            "owned store cleanup completed after exact readback"
        );
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn original_file_verifies_named_digest_and_small_body_limit() {
    let fixture = Fixture::new();
    let original = fixture.0.join("record");
    let bytes = b"original\0record\0\0";
    let id = object_id(bytes);
    write_record(&original, bytes);
    let verify = |id, cap| verify_owned_file(id, storage::open_read(&original).unwrap(), cap);
    assert_eq!(verify(id, bytes.len() as u64), Ok(()));
    assert_eq!(verify([9; 32], bytes.len() as u64), Err(Error::Integrity));
    assert_eq!(verify(id, bytes.len() as u64 - 1), Err(Error::Limit));
    let mut corrupt = fs::read(&original).unwrap();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&original, corrupt).unwrap();
    assert_eq!(verify(id, bytes.len() as u64), Err(Error::Integrity));
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn owned_receipts_retry_original_durability_after_ambiguous_rename() {
    let root = Fixture::new();
    let child = root.0.join("cache");
    private_dir(&child).unwrap();
    let resources = crate::resource_profile::ResourceProfile::SmallV1;
    let limits = resources.receiver_limits();
    let c = crate::wan::context([11; 32]);
    let ns = c.namespace().unwrap();
    let lease = OwnedStoreLease::capture(
        Arc::new(File::open(&root.0).unwrap()),
        "cache".as_ref(),
        Arc::new(File::open(&child).unwrap()),
        OwnedStorePolicy::small_v1(&limits, 102, OwnedLaneNamespaces::One(ns)).unwrap(),
    )
    .unwrap();
    let mut endpoint =
        Endpoint::open_at(&lease, limits, crate::wan::BenchAuthority, resources).unwrap();
    let body = b"original durable receipt\0\0";
    let offer = Offer::cold(body, Mode::Raw);
    let begin = Frame {
        kind: 1,
        transfer: [2; 32],
        seq: 0,
        body: offer.encode().unwrap(),
    };
    let mut probe = Frame {
        kind: 4,
        transfer: [3; 32],
        seq: 0,
        body: offer.id().unwrap().to_vec(),
    };
    probe.body.extend_from_slice(&begin.body);
    // The original staged file was synced and renamed, but the final directory
    // sync did not complete. Its discoverable name is insufficient for a receipt.
    lease.fail_next_publication_sync();
    assert_eq!(
        endpoint.publish(&c, ns, &offer, body, None),
        Err(Error::Storage)
    );
    assert_eq!(endpoint.read_object(&c, offer.object).unwrap(), body);
    for frame in [&probe, &begin] {
        lease.fail_next_publication_sync();
        assert_eq!(endpoint.handle(&c, frame, 1), Err(Error::Storage));
        let receipt = endpoint.handle(&c, frame, 1).unwrap();
        assert_eq!(
            (receipt.kind, receipt.body.as_slice()),
            (103, offer.object.as_slice())
        );
    }
    probe.transfer = begin.transfer;
    probe.seq = 1;
    lease.fail_next_publication_sync();
    assert_eq!(endpoint.handle(&c, &probe, 1), Err(Error::Storage));
    assert_eq!(endpoint.handle(&c, &probe, 1).unwrap().kind, 103);
    lease.fail_next_publication_sync();
    assert_eq!(
        endpoint.publish(&c, ns, &offer, body, None),
        Err(Error::Storage)
    );
    endpoint.publish(&c, ns, &offer, body, None).unwrap();
}
