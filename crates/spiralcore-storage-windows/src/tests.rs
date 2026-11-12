use crate::*;
use std::{
    cell::RefCell,
    ffi::c_void,
    fs,
    io::Write,
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{Authorization::*, *},
    Storage::FileSystem::{
        FILE_ADD_FILE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_LIST_DIRECTORY,
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static QUOTA_PINNED: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}
pub(crate) fn pending_requested() -> bool {
    std::env::var_os("SC_TEST_PENDING_QUOTA").is_some()
}
pub(crate) fn quota_namespace_pinned() {
    let action = QUOTA_PINNED.with(|slot| slot.borrow_mut().take());
    if let Some(action) = action {
        action();
    }
}
fn process_cpu() -> u64 {
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::Threading::{GetCurrentProcess, GetProcessTimes},
    };
    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut exit, mut kernel, mut user) = (creation, creation, creation);
    // SAFETY: current-process pseudo-handle and four valid output structures.
    assert_ne!(
        unsafe {
            GetProcessTimes(
                GetCurrentProcess(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        },
        0
    );
    let ticks =
        |value: FILETIME| (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime);
    ticks(kernel) + ticks(user)
}
fn timed<T>(action: impl FnOnce() -> T) -> (T, u128, u64) {
    let cpu = process_cpu();
    let began = Instant::now();
    let value = action();
    (value, began.elapsed().as_micros(), process_cpu() - cpu)
}
fn quota_comparison(root: &Path, namespace: &Path) {
    for index in 0..63 {
        let mut file = create_new_file(&namespace.join(format!("bench-{index}.object"))).unwrap();
        file.write_all(b"quota benchmark").unwrap();
        file.sync_all().unwrap();
    }
    let old = || {
        fs::read_dir(namespace)
            .unwrap()
            .map(|entry| {
                open_read(&entry.unwrap().path())
                    .unwrap()
                    .metadata()
                    .unwrap()
                    .len()
            })
            .sum::<u64>()
    };
    let batch = || quota_usage(root, "cache", 64, |name| name == "cache", "object").unwrap();
    let (mut old_us, mut new_us, mut old_cpu, mut new_cpu) = (0u128, 0u128, 0u64, 0u64);
    for round in 0..6 {
        let (before, after) = if round % 2 == 0 {
            let before = timed(old);
            (before, timed(batch))
        } else {
            let after = timed(batch);
            (timed(old), after)
        };
        assert_eq!((before.0, before.0, 64), after.0);
        old_us += before.1;
        new_us += after.1;
        old_cpu += before.2;
        new_cpu += after.2;
    }
    eprintln!("QUOTA_BATCH_COMPARE files=64 paired_rounds=6 old_elapsed_us={old_us} batch_elapsed_us={new_us} old_process_cpu_100ns={old_cpu} batch_process_cpu_100ns={new_cpu}");
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-windows-backend-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Acl {
    descriptor: *mut c_void,
    owner: PSID,
    dacl: *mut ACL,
}
impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: sole owner of the security descriptor returned by Win32.
        unsafe {
            LocalFree(self.descriptor);
        }
    }
}
impl Acl {
    fn read(path: &Path) -> Self {
        let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let (mut owner, mut dacl, mut descriptor) = (null_mut(), null_mut(), null_mut());
        // SAFETY: owned fixture path and writable security descriptor outputs.
        let code = unsafe {
            GetNamedSecurityInfoW(
                name.as_ptr(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(code, 0);
        assert!(!owner.is_null() && !dacl.is_null() && !descriptor.is_null());
        Self {
            descriptor,
            owner,
            dacl,
        }
    }
    fn owner_sid(&self) -> String {
        let mut text = null_mut();
        // SAFETY: owner SID belongs to this live security descriptor.
        assert_ne!(unsafe { ConvertSidToStringSidW(self.owner, &mut text) }, 0);
        let mut length = 0;
        // SAFETY: documented NUL-terminated SID output, bounded before use.
        while length < 256 && unsafe { *text.add(length) } != 0 {
            length += 1;
        }
        assert!(length < 256);
        // SAFETY: the observed prefix precedes the returned string's NUL.
        let result =
            String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) }).unwrap();
        // SAFETY: release exactly the API-allocated string.
        unsafe {
            LocalFree(text.cast());
        }
        result
    }
    fn bytes(&self) -> Vec<u8> {
        let mut control = 0;
        let mut revision = 0;
        // SAFETY: live valid descriptor and integer output buffers.
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(self.descriptor, &mut control, &mut revision) },
            0
        );
        // SAFETY: ACL header is kernel-populated within the retained descriptor.
        let size = usize::from(unsafe { (*self.dacl).AclSize });
        // SAFETY: AclSize bounds its complete valid buffer.
        let mut bytes =
            unsafe { std::slice::from_raw_parts(self.dacl.cast::<u8>(), size) }.to_vec();
        bytes.extend_from_slice(&control.to_le_bytes());
        bytes
    }
}
fn replace_acl(path: &Path, text: &str, protected: bool) {
    let sddl: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = null_mut();
    // SAFETY: fixed test SDDL and writable LocalAlloc output.
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        },
        0
    );
    let (mut present, mut defaulted, mut dacl) = (0, 0, null_mut());
    // SAFETY: successful conversion returned a live valid descriptor.
    assert_ne!(
        unsafe { GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) },
        0
    );
    assert_ne!(present, 0);
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let protection = if protected {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // SAFETY: mutate only the owned fixture's DACL; never its owner or parent.
    let code = unsafe {
        SetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | protection,
            null_mut(),
            null_mut(),
            dacl,
            null_mut(),
        )
    };
    // SAFETY: descriptor has no remaining borrows after the synchronous call.
    unsafe {
        LocalFree(descriptor);
    }
    assert_eq!(code, 0);
}

#[test]
fn broad_or_inherited_windows_acl_refuses_without_permission_repair() {
    if !pending_requested() {
        let sandbox = Fixture::new();
        private_dir(&sandbox.0).unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::broad_or_inherited_windows_acl_refuses_without_permission_repair",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SC_TEST_PENDING_QUOTA", "1")
            .env("TEMP", &sandbox.0)
            .env("TMP", &sandbox.0)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("QUOTA_UNEXPECTED_PENDING_FAIL_STOP")
        );
        let path = sandbox.0.clone();
        drop(sandbox);
        assert!(
            !path.exists(),
            "aborted child's owned sandbox must be retired"
        );
    }
    for inherited in [false, true] {
        let fixture = Fixture::new();
        private_dir(&fixture.0).unwrap();
        let owner = Acl::read(&fixture.0).owner_sid();
        let text = if inherited {
            format!("D:(A;;FA;;;{owner})")
        } else {
            "D:P(A;;FA;;;WD)".into()
        };
        replace_acl(&fixture.0, &text, !inherited);
        let before = Acl::read(&fixture.0).bytes();
        assert!(matches!(private_dir(&fixture.0), Err(Error::Storage)));
        assert!(
            Acl::read(&fixture.0).bytes() == before,
            "refusal must not repair the ACL"
        );
    }
    let fixture = Fixture::new();
    private_dir(&fixture.0).unwrap();
    let namespace = fixture.0.join("cache");
    private_dir(&namespace).unwrap();
    let path = namespace.join("one.object");
    let mut file = create_new_file(&path).unwrap();
    file.write_all(b"exact quota bytes").unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert_eq!(
        quota_usage(&fixture.0, "cache", 64, |name| name == "cache", "object").unwrap(),
        (17, 17, 1)
    );
    assert_eq!(
        quota_usage(&fixture.0, "other", 64, |name| name == "cache", "object").unwrap(),
        (0, 17, 1)
    );
    assert_eq!(
        quota_usage(&fixture.0, "cache", 0, |name| name == "cache", "object"),
        Err(Error::Limit)
    );
    for invalid in ["", "..", "../cache", "cache:stream", "cache.", "cache "] {
        assert!(quota_usage(&fixture.0, invalid, 64, |_| true, "object").is_err());
    }
    let owner = Acl::read(&path).owner_sid();
    for (text, protected) in [
        ("D:P(A;;FA;;;WD)".to_owned(), true),
        (format!("D:(A;;FA;;;{owner})"), false),
    ] {
        replace_acl(&path, &text, protected);
        let before = Acl::read(&path).bytes();
        assert!(quota_usage(&fixture.0, "cache", 64, |_| true, "object").is_err());
        assert_eq!(Acl::read(&path).bytes(), before, "no permission repair");
        replace_acl(&path, &format!("D:P(A;;FA;;;{owner})"), true);
    }
    let link = namespace.join("hardlink");
    fs::hard_link(&path, &link).unwrap();
    assert!(quota_usage(&fixture.0, "cache", 64, |_| true, "object").is_err());
    fs::remove_file(link).unwrap();
    let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
    assert!(quota_usage(&fixture.0, "cache", 64, |_| true, "object").is_err());
    drop(writer);
    let directory_writer = fs::OpenOptions::new()
        .access_mode(FILE_LIST_DIRECTORY | FILE_ADD_FILE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(&namespace)
        .unwrap();
    assert!(quota_usage(&fixture.0, "cache", 64, |_| true, "object").is_err());
    drop(directory_writer);
    let directory = namespace.join("directory.object");
    private_dir(&directory).unwrap();
    assert!(quota_usage(&fixture.0, "cache", 64, |_| true, "object").is_err());
    fs::remove_dir(directory).unwrap();
    let renamed = fixture.0.join("renamed");
    let from = namespace.clone();
    let to = renamed.clone();
    let sibling = fixture.0.with_extension("sibling");
    QUOTA_PINNED.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            let created = private_dir(&sibling);
            assert!(
                created.is_ok(),
                "unrelated sibling creation must stay available: {created:?}"
            );
            let moved = sibling.with_extension("moved");
            let mut payload = create_new_file(&sibling.join("owned")).unwrap();
            payload.write_all(b"independent sibling").unwrap();
            drop(payload);
            fs::rename(&sibling, &moved).unwrap();
            fs::remove_dir_all(moved).unwrap();
            assert!(
                fs::rename(from, to).is_err(),
                "original namespace must be pinned"
            );
        }))
    });
    assert_eq!(
        quota_usage(&fixture.0, "cache", 64, |_| true, "object").unwrap(),
        (17, 17, 1)
    );
    assert!(QUOTA_PINNED.with(|slot| slot.borrow().is_none()));
    fs::rename(&namespace, &renamed).unwrap();
    fs::rename(&renamed, &namespace).unwrap();
    let foreign = fixture.0.join("foreign");
    private_dir(&foreign).unwrap();
    let from = fixture.0.clone();
    let to = fixture.0.with_extension("renamed");
    QUOTA_PINNED.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            assert!(
                fs::rename(from, to).is_err(),
                "original root must be pinned"
            );
        }))
    });
    quota_usage(&fixture.0, "cache", 64, |_| true, "object").unwrap();
    let root_renamed = fixture.0.with_extension("renamed");
    fs::rename(&fixture.0, &root_renamed).unwrap();
    fs::rename(&root_renamed, &fixture.0).unwrap();
    // Change only this test worker's otherwise unimpersonated effective token.
    // The original process token must never become a fallback for anonymous.
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_NO_TOKEN},
        System::Threading::{GetCurrentThread, OpenThreadToken},
    };
    let mut token = null_mut();
    // SAFETY: current thread and writable owned-token output.
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) };
    if opened != 0 {
        // SAFETY: successful OpenThreadToken returned this owned handle.
        unsafe {
            CloseHandle(token);
        }
        panic!("fixture requires an unimpersonated test worker");
    }
    // SAFETY: reads the failure code from the immediately preceding query.
    assert_eq!(unsafe { GetLastError() }, ERROR_NO_TOKEN);
    struct Revert;
    impl Drop for Revert {
        fn drop(&mut self) {
            // SAFETY: restore exactly this originally unimpersonated worker.
            assert_ne!(unsafe { RevertToSelf() }, 0);
        }
    }
    let revert = Revert;
    QUOTA_PINNED.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(|| {
            // SAFETY: the isolated test worker owns its temporary anonymous token.
            assert_ne!(unsafe { ImpersonateAnonymousToken(GetCurrentThread()) }, 0);
        }))
    });
    let refused = quota_usage(&fixture.0, "cache", 64, |name| name == "cache", "object");
    drop(revert);
    assert!(QUOTA_PINNED.with(|slot| slot.borrow().is_none()));
    assert!(
        refused.is_err(),
        "never substitute process identity after token change"
    );
    assert_eq!(
        quota_usage(&fixture.0, "cache", 64, |name| name == "cache", "object").unwrap(),
        (17, 17, 1)
    );
    quota_comparison(&fixture.0, &namespace);
}

#[test]
fn windows_junction_at_root_or_ancestor_refuses_without_target_mutation() {
    let fixture = Fixture::new();
    private_dir(&fixture.0).unwrap();
    let target = fixture.0.join("target");
    private_dir(&target).unwrap();
    let junction = fixture.0.join("junction");
    let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // Only fixed fixture-relative operands enter this script. PowerShell's
    // junction operation avoids cmd.exe's separate builtin argument grammar.
    let output = std::process::Command::new(powershell)
        .current_dir(&fixture.0)
        .args([
            "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path 'junction' -Target (Get-Item -LiteralPath 'target').FullName | Out-Null",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "owned junction setup failed: status={:?}, stdout={:?}, stderr={:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(
        fs::symlink_metadata(&junction).unwrap().file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT,
        0,
        "owned junction setup must create an actual reparse point"
    );
    assert!(matches!(private_dir(&junction), Err(Error::Storage)));
    assert!(quota_usage(&junction, "cache", 64, |_| true, "object").is_err());
    assert!(matches!(
        private_dir(&junction.join("nested")),
        Err(Error::Storage)
    ));
    assert!(!target.join("nested").exists());
    assert!(fs::read_dir(&target).unwrap().next().is_none());
    fs::remove_dir(&junction).unwrap();
}

// Same-executable control: the previous production existing-file path creates
// a descriptor that CreateFileW ignores. Never used by production callers.
fn legacy_existing_read(path: &Path) -> Result<std::fs::File> {
    crate::windows::legacy_existing_read(path)
}
#[test]
fn existing_open_descriptor_elision_preserves_guards_and_records_paired_cost() {
    use std::io::Read;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};
    const FILES: usize = 64;
    const BYTES: usize = 16 * 1024;
    let fixture = Fixture::new();
    private_dir(&fixture.0).unwrap();
    let namespace = fixture.0.join("existing-open");
    private_dir(&namespace).unwrap();
    let mut records = Vec::with_capacity(FILES);
    let (_, setup_us, setup_cpu) = timed(|| {
        for index in 0..FILES {
            let mut bytes = vec![0x61; BYTES];
            bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
            bytes[BYTES - 1] = 0;
            let path = namespace.join(format!("{index}.object"));
            let mut file = create_new_file(&path).unwrap();
            file.write_all(&bytes).unwrap();
            file.sync_all().unwrap();
            drop(file);
            records.push((path, bytes));
        }
    });
    // Both paths check the same live private file and re-query its effective
    // token on every iteration. The control varies only descriptor acquisition.
    {
        let held = open_read(&records[0].0).unwrap();
        let compare = |inline: bool| {
            for _ in 0..2048 {
                let used_inline = if inline {
                    crate::windows::compare_descriptor_query::<true>(&held)
                } else {
                    crate::windows::compare_descriptor_query::<false>(&held)
                }
                .unwrap();
                assert_eq!(used_inline, inline);
            }
        };
        for trial in 0..6 {
            let (dynamic, bounded) = if trial % 2 == 0 {
                let old = timed(|| compare(false));
                (old, timed(|| compare(true)))
            } else {
                let new = timed(|| compare(true));
                (timed(|| compare(false)), new)
            };
            eprintln!("PRIVATE_ACL_PAIR trial={trial} checks=2048 dynamic_wall_us={} bounded_wall_us={} dynamic_process_cpu_100ns={} bounded_process_cpu_100ns={}",
                dynamic.1, bounded.1, dynamic.2, bounded.2);
        }
    }
    type Reader = fn(&Path) -> Result<std::fs::File>;
    let check = |read: Reader| {
        for (path, bytes) in &records {
            let mut file = read(path).unwrap();
            let mut actual = Vec::new();
            file.read_to_end(&mut actual).unwrap();
            assert_eq!(&actual, bytes, "exact length and original bytes");
        }
    };
    let (_, warm_us, warm_cpu) = timed(|| {
        check(legacy_existing_read);
        check(open_read);
    });
    let serialized: u64 = records
        .iter()
        .map(|(p, _)| fs::metadata(p).unwrap().len())
        .sum();
    assert_eq!(serialized, (FILES * BYTES) as u64);
    assert!(serialized < 2 * 1024 * 1024);
    eprintln!("EXISTING_OPEN_SETUP {{\"files\":{FILES},\"payload_bytes_each\":{BYTES},\"serialized_bytes\":{serialized},\"setup_wall_us\":{setup_us},\"setup_process_cpu_100ns\":{setup_cpu},\"warm_wall_us\":{warm_us},\"warm_process_cpu_100ns\":{warm_cpu},\"cold_cache_claim\":false,\"scope\":\"synthetic checked native opens and full exact reads; not network or MVM\"}}");
    for trial in 0..6 {
        let (old, candidate) = if trial % 2 == 0 {
            let old = timed(|| check(legacy_existing_read));
            (old, timed(|| check(open_read)))
        } else {
            let candidate = timed(|| check(open_read));
            (timed(|| check(legacy_existing_read)), candidate)
        };
        eprintln!("EXISTING_OPEN_PAIR {{\"trial\":{trial},\"order\":\"{}\",\"files\":{FILES},\"bytes\":{serialized},\"old_wall_us\":{},\"candidate_wall_us\":{},\"old_process_cpu_100ns\":{},\"candidate_process_cpu_100ns\":{},\"exact_readbacks\":true}}", if trial % 2 == 0 { "old-new" } else { "new-old" }, old.1, candidate.1, old.2, candidate.2);
    }
    for (path, _) in &records {
        let before = Acl::read(path);
        let owner = before.owner_sid();
        let descriptor = before.bytes();
        for read in [legacy_existing_read as Reader, open_read] {
            let file = read(path).unwrap();
            let mut flags = 0;
            // SAFETY: live file handle and valid writable flag output.
            assert_ne!(
                unsafe { GetHandleInformation(file.as_raw_handle(), &mut flags) },
                0
            );
            assert_eq!(flags & HANDLE_FLAG_INHERIT, 0, "non-inheritable handle");
            assert_eq!(Acl::read(path).owner_sid(), owner);
            assert_eq!(
                Acl::read(path).bytes(),
                descriptor,
                "no descriptor mutation"
            );
        }
    }
    let refuse = |path: &Path| {
        for read in [legacy_existing_read as Reader, open_read] {
            assert!(matches!(read(path), Err(Error::Storage)));
        }
    };
    refuse(&namespace.join("missing.object"));
    refuse(&namespace);
    let path = &records[0].0;
    let alias = namespace.join("hardlink.alias");
    fs::hard_link(path, &alias).unwrap();
    refuse(path);
    fs::remove_file(alias).unwrap();
    let writer = fs::OpenOptions::new().write(true).open(path).unwrap();
    refuse(path);
    drop(writer);
    let broad = namespace.join("broad.object");
    let mut file = create_new_file(&broad).unwrap();
    file.write_all(b"owned broad ACL guard").unwrap();
    drop(file);
    replace_acl(&broad, "D:P(A;;FA;;;WD)", true);
    let descriptor = Acl::read(&broad).bytes();
    let owner = Acl::read(&broad).owner_sid();
    refuse(&broad);
    assert_eq!(Acl::read(&broad).bytes(), descriptor, "no ACL repair");
    assert_eq!(Acl::read(&broad).owner_sid(), owner, "no owner repair");
    let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = std::process::Command::new(powershell)
        .current_dir(&fixture.0)
        .env_remove("PSModulePath")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path 'existing-junction' -Target (Get-Item -LiteralPath 'existing-open').FullName | Out-Null"])
        .output().unwrap();
    assert!(
        output.status.success(),
        "owned junction setup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let junction = fixture.0.join("existing-junction");
    assert_ne!(
        fs::symlink_metadata(&junction).unwrap().file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT,
        0
    );
    refuse(&junction.join(path.file_name().unwrap()));
    fs::remove_dir(junction).unwrap();
    check(legacy_existing_read);
    check(open_read);
    eprintln!("EXISTING_OPEN_GUARDS {{\"exact_readbacks\":true,\"noninherit\":true,\"owner_dacl_unchanged\":true,\"missing\":true,\"directory\":true,\"hardlink\":true,\"conflicting_writer\":true,\"broad_acl_no_repair\":true,\"reparse_ancestor\":true}}");
    let root = fixture.0.clone();
    drop(records);
    drop(fixture);
    assert!(
        !root.exists(),
        "owned synthetic store cleanup after verification"
    );
}
