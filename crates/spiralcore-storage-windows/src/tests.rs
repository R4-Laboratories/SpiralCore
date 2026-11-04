use crate::*;
use std::{
    ffi::c_void,
    fs,
    os::windows::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{Authorization::*, *},
    Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
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
}

#[test]
fn windows_junction_at_root_or_ancestor_refuses_without_target_mutation() {
    let fixture = Fixture::new();
    private_dir(&fixture.0).unwrap();
    let target = fixture.0.join("target");
    private_dir(&target).unwrap();
    let junction = fixture.0.join("junction");
    let cmd = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/cmd.exe");
    // Fixed relative operands avoid cmd.exe's separate path-quoting rules.
    // Enable extensions explicitly: mklink is a command-processor builtin.
    let output = std::process::Command::new(cmd)
        .current_dir(&fixture.0)
        .args([
            "/D", "/E:ON", "/V:OFF", "/C", "mklink", "/J", "junction", "target",
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
    assert!(matches!(
        private_dir(&junction.join("nested")),
        Err(Error::Storage)
    ));
    assert!(!target.join("nested").exists());
    assert!(fs::read_dir(&target).unwrap().next().is_none());
    fs::remove_dir(&junction).unwrap();
}
