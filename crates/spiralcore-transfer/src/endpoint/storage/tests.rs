use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> Result<()> {
        Ok(())
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
                "spiral-storage-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn private_store_refuses_hardlinked_profile_without_resetting_it() {
    let fixture = Fixture::new();
    drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
    let profile = fixture.0.join(".spiralcore-profile-v1");
    let original = fs::read(&profile).unwrap();
    fs::hard_link(&profile, fixture.0.join("untrusted-alias")).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    assert_eq!(fs::read(profile).unwrap(), original);
}

fn context() -> Context {
    Context {
        tenant: "storage-test".into(),
        domain: "local".into(),
        sender: "sender".into(),
        recipient: "receiver".into(),
        mailbox: "objects".into(),
        capability: "fixture".into(),
        authorization_scope: "exact-storage".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}

#[test]
fn published_bytes_survive_restart_and_fresh_read_rejects_corruption() {
    let fixture = Fixture::new();
    let context = context();
    let bytes = b"private exact storage\0readback";
    let id = object_id(bytes);
    let mut endpoint = Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    crate::client::transfer(bytes, Mode::Raw, None, [0x24; 32], |frame| {
        endpoint.handle(&context, frame, 0)
    })
    .unwrap();
    assert_eq!(endpoint.read_object(&context, id).unwrap(), bytes);
    drop(endpoint);
    let mut reopened = Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap();
    assert_eq!(reopened.read_object(&context, id).unwrap(), bytes);
    let publication = fixture
        .0
        .join(hex(&context.namespace().unwrap()))
        .join(format!("{}.object", hex(&id)));
    let mut record = fs::read(&publication).unwrap();
    *record.last_mut().unwrap() ^= 1;
    fs::write(publication, record).unwrap();
    assert_eq!(reopened.read_object(&context, id), Err(Error::Integrity));
}

#[test]
fn staging_collection_refuses_hardlinks_without_removing_either_name() {
    let fixture = Fixture::new();
    drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
    let namespace = fixture.0.join(hex(&context().namespace().unwrap()));
    storage::private_dir(&namespace).unwrap();
    let stage = namespace.join(".stage-123-0");
    let alias = fixture.0.join("stage-alias");
    drop(storage::create_new_file(&stage).unwrap());
    fs::hard_link(&stage, &alias).unwrap();
    assert!(matches!(
        Endpoint::open(&fixture.0, Limits::default(), Allow),
        Err(Error::Storage)
    ));
    assert!(stage.exists() && alias.exists());
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };

    #[test]
    fn original_drive_absolute_syntax_refuses_aliases_without_store_mutation() {
        use std::path::{Component, Prefix};
        let fixture = Fixture::new();
        drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
        let marker = fixture.0.join(".spiralcore-profile-v1");
        let original = fs::read(&marker).unwrap();
        let root_relative: PathBuf = fixture.0.components().skip(1).collect();
        let drive = match fixture.0.components().next().unwrap() {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
                _ => panic!("fixture must use a local drive"),
            },
            _ => panic!("fixture must be absolute"),
        };
        // Existing nonempty working directories cannot be initialized by the
        // old endpoint either. No global chdir or outside-fixture creation.
        for path in [
            PathBuf::from("."),
            PathBuf::from(format!("{}:", char::from(drive))),
            root_relative,
        ] {
            assert!(
                matches!(
                    Endpoint::open(&path, Limits::default(), Allow),
                    Err(Error::Unsupported)
                ),
                "original input must name its drive and root before IO"
            );
        }
        for suffix in [
            r"missing\..\child",
            r"candidate.",
            r"candidate ",
            r"missing.\child",
            r"missing \child",
        ] {
            assert!(Endpoint::open(&fixture.0.join(suffix), Limits::default(), Allow).is_err());
        }
        assert!(!fixture.0.join("child").exists());
        assert!(!fixture.0.join("candidate").exists());
        assert!(!fixture.0.join("missing").exists());
        assert_eq!(fs::read(marker).unwrap(), original);
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
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
                unsafe {
                    GetSecurityDescriptorControl(self.descriptor, &mut control, &mut revision)
                },
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
            unsafe {
                GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            },
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
        for inherited in [false, true] {
            let fixture = Fixture::new();
            drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
            let owner = Acl::read(&fixture.0).owner_sid();
            let text = if inherited {
                format!("D:(A;;FA;;;{owner})")
            } else {
                "D:P(A;;FA;;;WD)".into()
            };
            replace_acl(&fixture.0, &text, !inherited);
            let before = Acl::read(&fixture.0).bytes();
            assert!(matches!(
                Endpoint::open(&fixture.0, Limits::default(), Allow),
                Err(Error::Storage)
            ));
            assert!(
                Acl::read(&fixture.0).bytes() == before,
                "refusal must not repair the ACL"
            );
        }
    }

    #[test]
    fn windows_junction_at_root_or_ancestor_refuses_without_target_mutation() {
        let fixture = Fixture::new();
        drop(Endpoint::open(&fixture.0, Limits::default(), Allow).unwrap());
        let target = fixture.0.join("target");
        storage::private_dir(&target).unwrap();
        let junction = fixture.0.join("junction");
        let cmd = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/cmd.exe");
        let output = std::process::Command::new(cmd)
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success(), "owned junction setup failed");
        assert!(matches!(
            Endpoint::open(&junction, Limits::default(), Allow),
            Err(Error::Storage)
        ));
        assert!(matches!(
            Endpoint::open(&junction.join("nested"), Limits::default(), Allow),
            Err(Error::Storage)
        ));
        assert!(!target.join("nested").exists());
        assert!(fs::read_dir(&target).unwrap().next().is_none());
        fs::remove_dir(&junction).unwrap();
    }
}
