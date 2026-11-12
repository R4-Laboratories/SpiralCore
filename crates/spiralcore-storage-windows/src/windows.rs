use crate::{Error, Result};
use std::{
    ffi::c_void,
    fs::{self, File},
    mem::{size_of, size_of_val, zeroed, MaybeUninit},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr::{null, null_mut},
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::{
    Foundation::{
        GetLastError, LocalFree, ERROR_NO_TOKEN, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    },
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
    System::{
        SystemServices::FILE_PERSISTENT_ACLS,
        Threading::{GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken},
    },
};
mod quota;
pub use quota::quota_usage;

struct Local(*mut c_void);
impl Drop for Local {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: sole owner of a LocalAlloc-backed Win32 result.
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
// A Windows SID has at most fifteen 32-bit subauthorities and an eight-byte header.
struct Sid([u32; 17]);
impl Sid {
    fn ptr(&self) -> PSID {
        self.0.as_ptr().cast_mut().cast()
    }
    fn current() -> Result<Self> {
        let mut token = null_mut();
        // SAFETY: thread/process pseudo-handles and writable real token output.
        if unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) } == 0 {
            // Only absence of impersonation permits the process-token path.
            if unsafe { GetLastError() } != ERROR_NO_TOKEN
                || unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0
            {
                return Err(Error::Storage);
            }
        }
        // SAFETY: successful call returned one fresh real token handle.
        let token = unsafe { OwnedHandle::from_raw_handle(token) };
        // Query the same fresh effective token directly into the previous 4 KiB
        // ceiling. TokenUser writes only its actual result, including the SID.
        let mut buffer = [MaybeUninit::<usize>::uninit(); 4096 / size_of::<usize>()];
        let mut used = 0;
        // SAFETY: the aligned output has 4096 writable bytes for the kernel result.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                buffer.as_mut_ptr().cast(),
                size_of_val(&buffer) as u32,
                &mut used,
            )
        } == 0
            || !(size_of::<TOKEN_USER>()..=size_of_val(&buffer)).contains(&(used as usize))
        {
            return Err(Error::Storage);
        }
        // SAFETY: successful TokenUser query initialized this structure within used.
        let source = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
        let offset = (source as usize)
            .checked_sub(buffer.as_ptr() as usize)
            .ok_or(Error::Storage)?;
        let remaining = (used as usize).checked_sub(offset).ok_or(Error::Storage)?;
        if remaining < 8 || !(source as usize).is_multiple_of(4) {
            return Err(Error::Storage);
        }
        // SAFETY: the SID header is in bounds. Check the complete encoded length
        // before asking Windows to inspect the SID or copy it into aligned storage.
        let bytes = 8 + 4 * usize::from(unsafe { *source.cast::<u8>().add(1) });
        if bytes > 68 || bytes > remaining || unsafe { IsValidSid(source) } == 0 {
            return Err(Error::Storage);
        }
        let mut sid = Self([0; 17]);
        // SAFETY: both sides cover the length of the validated SID.
        if unsafe { CopySid(bytes as u32, sid.0.as_mut_ptr().cast(), source) } == 0 {
            return Err(Error::Storage);
        }
        Ok(sid)
    }
    fn descriptor(&self) -> Result<Local> {
        let mut text = null_mut();
        // SAFETY: valid retained SID and writable LocalAlloc string output.
        if unsafe { ConvertSidToStringSidW(self.ptr(), &mut text) } == 0 {
            return Err(Error::Storage);
        }
        let _text = Local(text.cast());
        let mut length = 0;
        // SAFETY: API guarantees a NUL-terminated SID string; bound its use.
        while length < 256 && unsafe { *text.add(length) } != 0 {
            length += 1;
        }
        if length == 256 {
            return Err(Error::Storage);
        }
        // SAFETY: these units precede the observed NUL in the returned string.
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
            .map_err(|_| Error::Storage)?;
        let sddl: Vec<u16> = format!("O:{sid}D:P(A;;FA;;;{sid})")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = null_mut();
        // SAFETY: fixed SDDL with a kernel-generated SID, live NUL-terminated input.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(Error::Storage);
        }
        Ok(Local(descriptor))
    }
}

fn normalized(path: &Path) -> Result<PathBuf> {
    // Resolve no caller-relative root: Endpoint retains its original path for
    // its lifetime, so every operation must name the same explicit drive/root.
    let mut original = path.components();
    if !matches!(original.next(),Some(Component::Prefix(prefix)) if matches!(prefix.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
        || !matches!(original.next(), Some(Component::RootDir))
    {
        return Err(Error::Unsupported);
    }
    // GetFullPathNameW may erase traversal/aliases. Check original UTF-16
    // segments before absolute() or any filesystem operation can do that.
    let raw: Vec<u16> = path.as_os_str().encode_wide().take(4097).collect();
    if raw.len() > 4096 {
        return Err(Error::Limit);
    }
    if raw.contains(&0)
        || raw
            .split(|c| *c == u16::from(b'\\') || *c == u16::from(b'/'))
            .any(|part| {
                part == [u16::from(b'.'), u16::from(b'.')]
                    || part
                        .last()
                        .is_some_and(|c| *c == u16::from(b'.') || *c == u16::from(b' '))
            })
    {
        return Err(Error::Storage);
    }
    let absolute = std::path::absolute(path)?;
    let mut parts = absolute.components();
    let drive = match parts.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive,
            _ => return Err(Error::Unsupported),
        },
        _ => return Err(Error::Unsupported),
    };
    if !matches!(parts.next(), Some(Component::RootDir)) {
        return Err(Error::Storage);
    }
    let mut result = PathBuf::from(format!(r"\\?\{}:\", char::from(drive)));
    let mut count = 0;
    for part in parts {
        let Component::Normal(name) = part else {
            return Err(Error::Storage);
        };
        let name = name.to_str().ok_or(Error::Storage)?;
        if name.contains([':', '\0'])
            || name.ends_with(['.', ' '])
            || name.encode_utf16().count() > 255
        {
            return Err(Error::Storage);
        }
        result.push(name);
        count += 1;
    }
    if count > 128 || result.as_os_str().encode_wide().count() > 4096 {
        return Err(Error::Limit);
    }
    Ok(result)
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn raw_open(
    path: &Path,
    access: u32,
    sharing: u32,
    creation: u32,
    security: *const SECURITY_ATTRIBUTES,
) -> Result<File> {
    let name = wide(path);
    // SAFETY: live terminated path, fixed flags, optional live security descriptor.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            sharing,
            security,
            creation,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(Error::Storage);
    }
    // SAFETY: successful CreateFileW transferred one fresh owned disk handle.
    Ok(unsafe { File::from_raw_handle(handle) })
}
fn information(file: &File, directory: bool) -> Result<BY_HANDLE_FILE_INFORMATION> {
    // SAFETY: plain Win32 integer structure, all-zero initial value is valid.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    // SAFETY: live borrowed handle and correctly sized writable output.
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
    {
        return Err(Error::Storage);
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory && info.nNumberOfLinks != 1)
    {
        return Err(Error::Storage);
    }
    Ok(info)
}
fn volume(file: &File, root: &Path) -> Result<()> {
    let name = wide(root);
    // SAFETY: fixed root path; GetDriveType reports the local drive category.
    if unsafe { GetDriveTypeW(name.as_ptr()) } != 3 {
        return Err(Error::Unsupported);
    }
    let mut flags = 0;
    let mut filesystem = [0u16; 16];
    // SAFETY: live directory and correctly sized filesystem-name output.
    if unsafe {
        GetVolumeInformationByHandleW(
            file.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            &mut flags,
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err(Error::Storage);
    }
    let length = filesystem
        .iter()
        .position(|c| *c == 0)
        .ok_or(Error::Storage)?;
    if flags & FILE_PERSISTENT_ACLS == 0
        || String::from_utf16_lossy(&filesystem[..length]) != "NTFS"
    {
        return Err(Error::Unsupported);
    }
    Ok(())
}
fn ancestors(path: &Path) -> Result<Vec<File>> {
    let mut parents: Vec<_> = path.ancestors().skip(1).collect();
    parents.reverse();
    let mut held = Vec::with_capacity(parents.len());
    for parent in parents {
        let file = raw_open(
            parent,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            null(),
        )?;
        information(&file, true)?;
        if held.is_empty() {
            volume(&file, parent)?;
        }
        held.push(file);
    }
    Ok(held)
}
fn private(file: &File, directory: bool, sid: &Sid) -> Result<()> {
    private_checked::<true>(file, directory, sid).map(|_| ())
}

#[cfg(test)]
pub(crate) fn compare_descriptor_query<const INLINE: bool>(file: &File) -> Result<bool> {
    private_checked::<INLINE>(file, false, &Sid::current()?)
}

fn private_checked<const INLINE: bool>(file: &File, directory: bool, sid: &Sid) -> Result<bool> {
    information(file, directory)?;
    // Ask for the same OWNER|DACL from this live handle in one bounded call.
    // NTFS normally returns a small self-relative descriptor for our one-ACE
    // layout. Keep the original Win32 path for descriptors needing more space.
    let mut buffer = [MaybeUninit::<usize>::uninit(); 512 / size_of::<usize>()];
    let mut used = 0;
    let mut owner = null_mut();
    let mut dacl = null_mut();
    let mut descriptor = buffer.as_mut_ptr().cast();
    let _allocated;
    let info = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    // SAFETY: the aligned buffer is writable for its entire advertised extent.
    let inline_used = INLINE
        && unsafe {
            GetKernelObjectSecurity(
                file.as_raw_handle(),
                info,
                descriptor,
                size_of_val(&buffer) as u32,
                &mut used,
            )
        } != 0;
    if inline_used {
        if !(size_of::<SECURITY_DESCRIPTOR_RELATIVE>()..=size_of_val(&buffer))
            .contains(&(used as usize))
        {
            return Err(Error::Storage);
        }
        let mut defaulted = 0;
        let mut present = 0;
        // SAFETY: successful kernel output is a live self-relative descriptor.
        if unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted) } == 0
            || unsafe {
                GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            } == 0
            || present == 0
        {
            return Err(Error::Storage);
        }
    } else {
        descriptor = null_mut();
        // SAFETY: this is the existing checked, dynamically-sized security query.
        let code = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                info,
                &mut owner,
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        };
        _allocated = Local(descriptor);
        if code != 0 {
            return Err(Error::Storage);
        }
    }
    if owner.is_null() || dacl.is_null() {
        return Err(Error::Storage);
    }
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: descriptor and its owner/ACL remain live through all checks below.
    if unsafe { EqualSid(owner, sid.ptr()) } == 0
        || unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0
        || control & SE_DACL_PROTECTED == 0
    {
        return Err(Error::Storage);
    }
    // SAFETY: output is a plain integer Win32 structure.
    let mut size: ACL_SIZE_INFORMATION = unsafe { zeroed() };
    // SAFETY: valid returned ACL and correctly sized information buffer.
    if unsafe {
        GetAclInformation(
            dacl,
            (&mut size as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
        || size.AceCount != 1
    {
        return Err(Error::Storage);
    }
    let mut entry = null_mut();
    // SAFETY: the ACL reports exactly one entry; output points inside its buffer.
    if unsafe { GetAce(dacl, 0, &mut entry) } == 0 || entry.is_null() {
        return Err(Error::Storage);
    }
    // SAFETY: ACL entry header exists; validate its type/size before its body.
    let header = unsafe { &*entry.cast::<ACE_HEADER>() };
    if header.AceType != 0
        || header.AceFlags != 0
        || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>()
    {
        return Err(Error::Storage);
    }
    // SAFETY: checked allow ACE header/body; SidStart addresses its SID payload.
    let allowed = unsafe { &*entry.cast::<ACCESS_ALLOWED_ACE>() };
    let allowed_sid: PSID = (&allowed.SidStart as *const u32).cast_mut().cast();
    let capacity = usize::from(header.AceSize) - std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
    if capacity < 8 {
        return Err(Error::Storage);
    }
    // SAFETY: the ACE contains at least the eight-byte SID header. Its count
    // determines the complete SID size before any SID API reads that body.
    let authorities = unsafe { *allowed_sid.cast::<u8>().add(1) };
    if 8 + usize::from(authorities) * 4 > capacity {
        return Err(Error::Storage);
    }
    // SAFETY: kernel-provided ACE; validate SID before comparing it.
    if allowed.Mask != FILE_ALL_ACCESS
        || unsafe { IsValidSid(allowed_sid) } == 0
        || unsafe { EqualSid(allowed_sid, sid.ptr()) } == 0
    {
        return Err(Error::Storage);
    }
    Ok(inline_used)
}
fn open(path: &Path, access: u32, sharing: u32, creation: u32) -> Result<File> {
    let path = normalized(path)?;
    let _parents = ancestors(&path)?;
    let sid = Sid::current()?;
    let file = if creation == OPEN_EXISTING {
        // CreateFileW ignores creation descriptors for existing files. NULL
        // attributes also retain the original non-inheritable handle policy.
        // Fresh identity and the consumed-handle checks below remain mandatory.
        raw_open(&path, access | READ_CONTROL, sharing, creation, null())?
    } else {
        let descriptor = sid.descriptor()?;
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        raw_open(&path, access | READ_CONTROL, sharing, creation, &security)?
    };
    private(&file, false, &sid)?;
    Ok(file)
}
#[cfg(test)]
pub(crate) fn legacy_existing_read(path: &Path) -> Result<File> {
    let path = normalized(path)?;
    let _parents = ancestors(&path)?;
    let sid = Sid::current()?;
    let descriptor = sid.descriptor()?;
    let security = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let file = raw_open(
        &path,
        GENERIC_READ | READ_CONTROL,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        &security,
    )?;
    private(&file, false, &sid)?;
    Ok(file)
}
pub fn private_file(path: &Path) -> Result<()> {
    open(
        path,
        FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
    )
    .map(drop)
}
pub fn open_read(path: &Path) -> Result<File> {
    open(path, GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING)
}
pub fn create_new_file(path: &Path) -> Result<File> {
    open(
        path,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ,
        CREATE_NEW,
    )
}
pub fn open_owner(path: &Path) -> Result<File> {
    open(
        path,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_ALWAYS,
    )
}

fn moved(source: &Path, destination: &Path) -> Result<()> {
    // SAFETY: fixed same-parent terminated paths. No replace/copy/reboot fallback.
    if unsafe {
        MoveFileExW(
            wide(source).as_ptr(),
            wide(destination).as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(Error::Storage);
    }
    Ok(())
}
fn object(info: &BY_HANDLE_FILE_INFORMATION) -> (u32, u32, u32) {
    (
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    )
}
fn dispose(file: File) -> Result<()> {
    let mut disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: the live handle was opened with DELETE; disposition applies to
    // that exact object, not a pathname that might subsequently change.
    if unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&mut disposition as *mut FILE_DISPOSITION_INFO).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    } == 0
    {
        return Err(Error::Storage);
    }
    Ok(())
}
pub fn private_dir(path: &Path) -> Result<()> {
    let path = normalized(path)?;
    if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    {
        let parent = path.parent().ok_or(Error::Storage)?;
        if !parent.exists() {
            private_dir(parent)?;
        }
        let _parents = ancestors(&path)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stage = parent.join(format!(
            ".spiral-new-directory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let sid = Sid::current()?;
        let descriptor = sid.descriptor()?;
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        // SAFETY: exclusive new private directory with a live protected descriptor.
        if unsafe { CreateDirectoryW(wide(&stage).as_ptr(), &security) } == 0 {
            return Err(Error::Storage);
        }
        let created = raw_open(
            &stage,
            FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            null(),
        )?;
        private(&created, true, &sid)?;
        let id = object(&information(&created, true)?);
        drop(created);
        if let Err(error) = moved(&stage, &path) {
            if let Ok(file) = raw_open(
                &stage,
                DELETE | FILE_READ_ATTRIBUTES | READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                OPEN_EXISTING,
                null(),
            ) {
                let own_stage = private(&file, true, &sid).is_ok()
                    && information(&file, true).is_ok_and(|actual| object(&actual) == id);
                if own_stage {
                    let _ = dispose(file);
                }
            }
            return Err(error);
        }
    }
    let _parents = ancestors(&path)?;
    let file = raw_open(
        &path,
        FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
        null(),
    )?;
    private(&file, true, &Sid::current()?)
}
pub fn publish(stage: &Path, destination: &Path, root: &Path) -> Result<()> {
    let stage = normalized(stage)?;
    let destination = normalized(destination)?;
    if stage.parent() != destination.parent() {
        return Err(Error::Storage);
    }
    private_dir(root)?;
    let _parents = ancestors(&destination)?;
    let before = information(&open_read(&stage)?, false)?;
    moved(&stage, &destination)?;
    let after = information(&open_read(&destination)?, false)?;
    if object(&before) != object(&after) {
        return Err(Error::Storage);
    }
    Ok(())
}
pub fn sync_existing(path: &Path) -> Result<()> {
    let file = open(
        path,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ,
        OPEN_EXISTING,
    )?;
    file.sync_all()?;
    Ok(())
}
pub fn remove_stage(path: &Path) -> Result<()> {
    let file = open(
        path,
        DELETE | FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        OPEN_EXISTING,
    )?;
    dispose(file)
}
