//! One synchronous, metadata-only scan of pinned private NTFS directories.
use super::*;
use std::{ffi::OsStr, marker::PhantomData, rc::Rc};

const DIRECTORY_ACCESS: u32 = FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL;
const FINAL_NAME_UNITS: usize = 4160;

fn final_name(file: &File) -> Result<PathBuf> {
    let mut buffer = vec![0u16; FINAL_NAME_UNITS];
    // SAFETY: live disk handle and bounded writable UTF-16 output buffer.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            FILE_NAME_NORMALIZED | VOLUME_NAME_GUID,
        )
    } as usize;
    if length == 0 || length >= buffer.len() || buffer[..length].contains(&0) {
        return Err(Error::Storage);
    }
    let text = String::from_utf16(&buffer[..length]).map_err(|_| Error::Storage)?;
    let rest = text.strip_prefix(r"\\?\Volume{").ok_or(Error::Storage)?;
    let (guid, suffix) = rest.split_once('}').ok_or(Error::Storage)?;
    if guid.len() != 36 || !suffix.starts_with('\\') {
        return Err(Error::Storage);
    }
    for (index, byte) in guid.bytes().enumerate() {
        if if [8, 13, 18, 23].contains(&index) {
            byte != b'-'
        } else {
            !byte.is_ascii_hexdigit()
        } {
            return Err(Error::Storage);
        }
    }
    Ok(PathBuf::from(text))
}

fn child_name(name: &OsStr) -> Result<&str> {
    let text = name.to_str().ok_or(Error::Storage)?;
    if text.is_empty()
        || text == "."
        || text.contains([':', '\\', '/', '\0'])
        || text.ends_with(['.', ' '])
        || text.encode_utf16().count() > 255
    {
        return Err(Error::Storage);
    }
    Ok(text)
}

fn current_sid(original: &Sid) -> Result<Sid> {
    let current = Sid::current()?;
    // SAFETY: both SIDs were copied from validated live token information.
    if unsafe { EqualSid(original.ptr(), current.ptr()) } == 0 {
        return Err(Error::Storage);
    }
    Ok(current)
}

fn pin(path: &Path) -> Result<File> {
    // LIST_DIRECTORY participates in sharing checks. Attribute-only handles
    // cannot establish the required rename/write exclusion.
    let file = raw_open(
        path,
        DIRECTORY_ACCESS,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        null(),
    )?;
    information(&file, true)?;
    Ok(file)
}

struct QuotaRoot {
    pins: Vec<File>,
    canonical: PathBuf,
    volume: u32,
    depth: usize,
    units: usize,
    sid: Sid,
    _thread: PhantomData<Rc<()>>,
}
impl QuotaRoot {
    fn open(path: &Path) -> Result<Self> {
        let path = normalized(path)?;
        let sid = Sid::current()?;
        let mut names: Vec<_> = path.ancestors().collect();
        names.reverse();
        let mut pins = Vec::with_capacity(names.len());
        let mut canonical = PathBuf::new();
        let mut serial = None;
        for name in names {
            let file = pin(name)?;
            let info = information(&file, true)?;
            let actual = final_name(&file)?;
            if pins.is_empty() {
                volume(&file, name)?;
                if actual.parent().is_some() {
                    return Err(Error::Unsupported);
                }
                serial = Some(info.dwVolumeSerialNumber);
            } else if actual.parent() != Some(canonical.as_path())
                || serial != Some(info.dwVolumeSerialNumber)
            {
                return Err(Error::Storage);
            }
            canonical = actual;
            pins.push(file);
        }
        let current = current_sid(&sid)?;
        private(pins.last().ok_or(Error::Storage)?, true, &current)?;
        Ok(Self {
            depth: path
                .components()
                .filter(|part| matches!(part, Component::Normal(_)))
                .count(),
            units: path.as_os_str().encode_wide().count(),
            pins,
            canonical,
            volume: serial.ok_or(Error::Storage)?,
            sid,
            _thread: PhantomData,
        })
    }
    fn check(&self) -> Result<Sid> {
        let sid = current_sid(&self.sid)?;
        for pin in &self.pins {
            if information(pin, true)?.dwVolumeSerialNumber != self.volume {
                return Err(Error::Storage);
            }
        }
        private(self.pins.last().ok_or(Error::Storage)?, true, &sid)?;
        Ok(sid)
    }
    fn namespace(&self, name: &OsStr) -> Result<QuotaNamespace<'_>> {
        let name = child_name(name)?;
        let units = self.units + 1 + name.encode_utf16().count();
        if self.depth + 2 > 128 || units > 4096 {
            return Err(Error::Limit);
        }
        let sid = self.check()?;
        let file = pin(&self.canonical.join(name))?;
        private(&file, true, &sid)?;
        let canonical = final_name(&file)?;
        if canonical.parent() != Some(self.canonical.as_path())
            || canonical.file_name() != Some(OsStr::new(name))
            || information(&file, true)?.dwVolumeSerialNumber != self.volume
        {
            return Err(Error::Storage);
        }
        Ok(QuotaNamespace {
            root: self,
            file,
            canonical,
            units,
        })
    }
}

struct QuotaNamespace<'a> {
    root: &'a QuotaRoot,
    file: File,
    canonical: PathBuf,
    units: usize,
}
impl QuotaNamespace<'_> {
    fn length(&self, name: &OsStr) -> Result<u64> {
        let name = child_name(name)?;
        if self.units + 1 + name.encode_utf16().count() > 4096 {
            return Err(Error::Limit);
        }
        let sid = self.root.check()?;
        private(&self.file, true, &sid)?;
        let file = raw_open(
            &self.canonical.join(name),
            GENERIC_READ | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            null(),
        )?;
        private(&file, false, &sid)?;
        let actual = final_name(&file)?;
        if actual.parent() != Some(self.canonical.as_path())
            || actual.file_name() != Some(OsStr::new(name))
            || information(&file, false)?.dwVolumeSerialNumber != self.root.volume
        {
            return Err(Error::Storage);
        }
        let length = file.metadata()?.len();
        current_sid(&self.root.sid)?;
        Ok(length)
    }
}

/// Count matching private files through one pinned root and namespace at a time.
/// The selector supplies layout only, never authorization. No handle, path or
/// data reader escapes. Filesystem observations remain authoritative each call.
pub fn quota_usage(
    root: &Path,
    scoped_namespace: &str,
    max_files: usize,
    namespace: fn(&str) -> bool,
    extension: &str,
) -> Result<(u64, u64, usize)> {
    child_name(OsStr::new(scoped_namespace))?;
    child_name(OsStr::new(extension))?;
    let root = QuotaRoot::open(root)?;
    let (mut scoped, mut all, mut count) = (0u64, 0u64, 0usize);
    for entry in fs::read_dir(&root.canonical)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(text) = name.to_str() else { continue };
        if !namespace(text) || !entry.file_type()?.is_dir() {
            continue;
        }
        let directory = root.namespace(&name)?;
        #[cfg(test)]
        super::super::tests::quota_namespace_pinned();
        for file in fs::read_dir(&directory.canonical)? {
            let file = file?;
            let name = file.file_name();
            if Path::new(&name)
                .extension()
                .is_none_or(|value| value != extension)
            {
                continue;
            }
            count = count.checked_add(1).ok_or(Error::Limit)?;
            if count > max_files {
                return Err(Error::Limit);
            }
            let length = directory.length(&name)?;
            all = all.checked_add(length).ok_or(Error::Limit)?;
            if text == scoped_namespace {
                scoped = scoped.checked_add(length).ok_or(Error::Limit)?;
            }
        }
    }
    root.check()?;
    Ok((scoped, all, count))
}
