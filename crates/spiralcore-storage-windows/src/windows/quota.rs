//! Synchronous accounting through owned directory handles, never ambient children.
use super::*;
use std::{marker::PhantomData, rc::Rc};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            ERROR_NO_MORE_FILES, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, STATUS_PENDING,
            UNICODE_STRING,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};
const DIRECTORY_ACCESS: u32 =
    FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE;
const ENUM_WORDS: usize = 2048;

fn child_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name.contains([':', '\\', '/', '\0'])
        || name.ends_with(['.', ' '])
        || name.encode_utf16().count() > 255
    {
        return Err(Error::Storage);
    }
    Ok(())
}
fn current_sid(original: &Sid) -> Result<Sid> {
    let current = Sid::current()?;
    // SAFETY: both SIDs were copied from validated token information.
    if unsafe { EqualSid(original.ptr(), current.ptr()) } == 0 {
        return Err(Error::Storage);
    }
    Ok(current)
}
fn require_completed(status: i32) {
    if status == STATUS_PENDING {
        // A pending request may still reference caller storage. This profile
        // requests completed synchronous NTFS opens; never unwind uncertain I/O.
        std::process::abort();
    }
}

struct Entry {
    name: String,
    directory: bool,
    id: u64,
}
fn each_entry(directory: &File, mut inspect: impl FnMut(Entry) -> Result<()>) -> Result<()> {
    let mut buffer = vec![0u64; ENUM_WORDS];
    let capacity = buffer.len() * size_of::<u64>();
    let prefix = std::mem::offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
    let mut restart = true;
    loop {
        buffer.fill(0);
        let class = if restart {
            FileIdBothDirectoryRestartInfo
        } else {
            FileIdBothDirectoryInfo
        };
        // SAFETY: private live directory and initialized, 8-byte-aligned bounded buffer.
        let success = unsafe {
            GetFileInformationByHandleEx(
                directory.as_raw_handle(),
                class,
                buffer.as_mut_ptr().cast(),
                capacity as u32,
            )
        };
        if success == 0 {
            // SAFETY: read the immediately preceding failed API call's error.
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                Ok(())
            } else {
                Err(Error::Storage)
            };
        }
        restart = false;
        let mut offset = 0usize;
        loop {
            if !offset.is_multiple_of(std::mem::align_of::<FILE_ID_BOTH_DIR_INFO>())
                || offset
                    .checked_add(size_of::<FILE_ID_BOTH_DIR_INFO>())
                    .is_none_or(|end| end > capacity)
            {
                return Err(Error::Storage);
            }
            // SAFETY: alignment and full fixed structure extent were checked; all fields are integers.
            let record = unsafe {
                &*buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>()
            };
            let bytes = record.FileNameLength as usize;
            let next = record.NextEntryOffset as usize;
            let end = offset
                .checked_add(prefix)
                .and_then(|start| start.checked_add(bytes))
                .ok_or(Error::Storage)?;
            if bytes == 0
                || !bytes.is_multiple_of(2)
                || bytes > 510
                || end > capacity
                || (next != 0
                    && (!next.is_multiple_of(8)
                        || next < prefix + bytes
                        || offset
                            .checked_add(next)
                            .is_none_or(|limit| limit >= capacity)))
            {
                return Err(Error::Storage);
            }
            // SAFETY: the bounded even filename extent follows the aligned fixed prefix.
            let units = unsafe {
                std::slice::from_raw_parts(
                    buffer
                        .as_ptr()
                        .cast::<u8>()
                        .add(offset + prefix)
                        .cast::<u16>(),
                    bytes / 2,
                )
            };
            let name = String::from_utf16(units).map_err(|_| Error::Storage)?;
            if name != "." && name != ".." {
                child_name(&name)?;
                inspect(Entry {
                    name,
                    directory: record.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                    id: record.FileId as u64,
                })?;
            }
            if next == 0 {
                break;
            }
            offset += next;
        }
    }
}
fn relative(parent: &File, entry: &Entry, directory: bool, serial: u32, sid: &Sid) -> Result<File> {
    child_name(&entry.name)?;
    if entry.id == 0 {
        return Err(Error::Storage);
    }
    let mut units: Vec<u16> = entry.name.encode_utf16().collect();
    let name = UNICODE_STRING {
        Length: (units.len() * 2) as u16,
        MaximumLength: (units.len() * 2) as u16,
        Buffer: units.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: null(),
        SecurityQualityOfService: null(),
    };
    let mut handle = null_mut();
    // SAFETY: all-zero IO_STATUS_BLOCK is valid writable API output storage.
    let mut status_block: IO_STATUS_BLOCK = unsafe { zeroed() };
    let access = if directory {
        DIRECTORY_ACCESS
    } else {
        GENERIC_READ | READ_CONTROL
    };
    // SAFETY: generated ABI types and live single-component input buffers; RootDirectory is retained.
    // SYNCHRONOUS_IO_NONALERT + SYNCHRONIZE requests a completed synchronous local-NTFS open.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access | SYNCHRONIZE,
            &attributes,
            &mut status_block,
            null(),
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ,
            FILE_OPEN,
            FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_REPARSE_POINT
                | if directory {
                    FILE_DIRECTORY_FILE
                } else {
                    FILE_NON_DIRECTORY_FILE
                },
            null(),
            0,
        )
    };
    require_completed(status);
    #[cfg(test)]
    if super::super::tests::pending_requested() {
        // The actual call has completed before simulating the invariant.
        eprintln!("QUOTA_UNEXPECTED_PENDING_FAIL_STOP");
        require_completed(STATUS_PENDING);
    }
    if status < 0 || handle.is_null() || handle == INVALID_HANDLE_VALUE {
        #[cfg(test)]
        eprintln!("QUOTA_NT_OPEN_REFUSAL directory={directory} status={status:#x}");
        return Err(Error::Storage);
    }
    // SAFETY: successful synchronous NtCreateFile transferred one fresh owned user handle.
    let file = unsafe { File::from_raw_handle(handle) };
    private(&file, directory, sid)?;
    let actual = information(&file, directory)?;
    let id = (u64::from(actual.nFileIndexHigh) << 32) | u64::from(actual.nFileIndexLow);
    if actual.dwVolumeSerialNumber != serial || id != entry.id {
        return Err(Error::Storage);
    }
    Ok(file)
}
struct QuotaRoot {
    file: File,
    serial: u32,
    depth: usize,
    units: usize,
    sid: Sid,
    _thread: PhantomData<Rc<()>>,
}
impl QuotaRoot {
    fn open(path: &Path) -> Result<Self> {
        let path = normalized(path)?;
        let sid = Sid::current()?;
        // Preserve the existing path-capture boundary. Shared ancestors get no new write exclusion.
        let parents = ancestors(&path)?;
        let file = raw_open(
            &path,
            DIRECTORY_ACCESS,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            null(),
        )?;
        let info = information(&file, true)?;
        if let Some(first) = parents.first() {
            if information(first, true)?.dwVolumeSerialNumber != info.dwVolumeSerialNumber {
                return Err(Error::Storage);
            }
        } else {
            volume(&file, &path)?;
        }
        private(&file, true, &current_sid(&sid)?)?;
        drop(parents);
        Ok(Self {
            file,
            serial: info.dwVolumeSerialNumber,
            depth: path
                .components()
                .filter(|part| matches!(part, Component::Normal(_)))
                .count(),
            units: path.as_os_str().encode_wide().count(),
            sid,
            _thread: PhantomData,
        })
    }
    fn check(&self) -> Result<Sid> {
        let sid = current_sid(&self.sid)?;
        private(&self.file, true, &sid)?;
        Ok(sid)
    }
}
/// Rescan private files through the captured root and namespace handles.
/// Selectors describe layout, not permission. Only checked scalar totals escape.
///
/// # Process termination
/// This local-NTFS profile requires completed synchronous opens. An unexpected
/// pending status aborts the process without unwinding potentially live I/O
/// storage; this function does not provide asynchronous-open recovery.
pub fn quota_usage(
    root: &Path,
    scoped_namespace: &str,
    max_files: usize,
    namespace: fn(&str) -> bool,
    extension: &str,
) -> Result<(u64, u64, usize)> {
    child_name(scoped_namespace)?;
    child_name(extension)?;
    let root = QuotaRoot::open(root)?;
    let (mut scoped, mut all, mut count) = (0u64, 0u64, 0usize);
    each_entry(&root.file, |entry| {
        if !namespace(&entry.name) {
            return Ok(());
        }
        if !entry.directory {
            return Err(Error::Storage);
        }
        if root.depth + 2 > 128 {
            return Err(Error::Limit);
        }
        let units = root.units + 1 + entry.name.encode_utf16().count();
        if units > 4096 {
            return Err(Error::Limit);
        }
        let directory = relative(&root.file, &entry, true, root.serial, &root.check()?)?;
        #[cfg(test)]
        super::super::tests::quota_namespace_pinned();
        each_entry(&directory, |file| {
            if Path::new(&file.name)
                .extension()
                .is_none_or(|value| value != extension)
            {
                return Ok(());
            }
            count = count.checked_add(1).ok_or(Error::Limit)?;
            if count > max_files || units + 1 + file.name.encode_utf16().count() > 4096 {
                return Err(Error::Limit);
            }
            let sid = root.check()?;
            private(&directory, true, &sid)?;
            let opened = relative(&directory, &file, false, root.serial, &sid)?;
            let length = opened.metadata()?.len();
            all = all.checked_add(length).ok_or(Error::Limit)?;
            if entry.name == scoped_namespace {
                scoped = scoped.checked_add(length).ok_or(Error::Limit)?;
            }
            current_sid(&root.sid)?;
            Ok(())
        })
    })?;
    root.check()?;
    Ok((scoped, all, count))
}
