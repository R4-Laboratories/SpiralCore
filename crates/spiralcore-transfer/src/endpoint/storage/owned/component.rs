//! Exact bounded name bytes, including a NUL slot for syscall arguments.
use crate::{Error, Id, Result};
use std::{
    ffi::{CStr, OsStr},
    fmt,
    os::unix::ffi::OsStrExt,
};

const CAPACITY: usize = 1024;
pub(crate) struct Component {
    bytes: [u8; CAPACITY],
    len: usize,
}
impl Component {
    pub(crate) fn new(name: &OsStr) -> Result<Self> {
        super::io::component(name)?;
        let mut result = Self {
            bytes: [0; CAPACITY],
            len: name.len(),
        };
        result.bytes[..result.len].copy_from_slice(name.as_bytes());
        Ok(result)
    }
    pub(crate) fn formatted(args: fmt::Arguments<'_>) -> Result<Self> {
        use fmt::Write;
        let mut result = Self {
            bytes: [0; CAPACITY],
            len: 0,
        };
        result.write_fmt(args).map_err(|_| Error::Limit)?;
        super::io::component(result.as_os_str())?;
        Ok(result)
    }
    pub(crate) fn id(id: &Id) -> Self {
        let mut result = Self {
            bytes: [0; CAPACITY],
            len: 64,
        };
        result.bytes[..64].copy_from_slice(&crate::hex_bytes(id));
        result
    }
    pub(crate) fn object(id: &Id) -> Self {
        let mut result = Self::id(id);
        result.bytes[64..71].copy_from_slice(b".object");
        result.len = 71;
        result
    }
    pub(crate) fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes(&self.bytes[..self.len])
    }
    pub(crate) fn as_c_str(&self) -> Result<&CStr> {
        CStr::from_bytes_with_nul(&self.bytes[..=self.len]).map_err(|_| Error::Storage)
    }
}
impl AsRef<OsStr> for Component {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}
impl fmt::Write for Component {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        if end >= CAPACITY {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        self.bytes[self.len] = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_names_preserve_raw_bytes_hex_and_numeric_stages() {
        let bytes = &[0x80, 0xff, b'x'];
        let name = Component::new(OsStr::from_bytes(bytes)).unwrap();
        assert_eq!(name.as_os_str().as_bytes(), bytes);
        assert_eq!(name.as_c_str().unwrap().to_bytes(), bytes);
        let id = std::array::from_fn(|at| at as u8);
        assert_eq!(
            Component::id(&id).as_c_str().unwrap().to_bytes(),
            crate::hex(&id).as_bytes()
        );
        assert_eq!(
            Component::object(&id).as_c_str().unwrap().to_bytes(),
            format!("{}.object", crate::hex(&id)).as_bytes()
        );
        let stage = Component::formatted(format_args!(".stage-{}-{}", u32::MAX, u64::MAX)).unwrap();
        assert_eq!(
            stage.as_c_str().unwrap().to_bytes(),
            b".stage-4294967295-18446744073709551615"
        );
    }
    #[test]
    fn component_ceiling_and_forbidden_bytes_refuse_without_translation() {
        let max = if cfg!(target_os = "macos") { 1023 } else { 255 };
        let exact = vec![b'x'; max];
        assert_eq!(
            Component::new(OsStr::from_bytes(&exact))
                .unwrap()
                .as_c_str()
                .unwrap()
                .to_bytes(),
            exact
        );
        for bad in [&b""[..], b".", b"..", b"a/b", b"/absolute", b"a\0b"] {
            assert!(matches!(
                Component::new(OsStr::from_bytes(bad)),
                Err(Error::Storage)
            ));
        }
        let too_long = vec![b'x'; max + 1];
        assert!(matches!(
            Component::new(OsStr::from_bytes(&too_long)),
            Err(Error::Storage)
        ));
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn long_unicode_component_is_an_exact_borrowed_c_string() {
        let value = "\u{0800}".repeat(128);
        assert_eq!(value.len(), 384);
        let name = Component::new(value.as_ref()).unwrap();
        assert_eq!(name.as_c_str().unwrap().to_bytes(), value.as_bytes());
    }
}
