use crate::{dag::Manifest, hash, Error, Id, Result, MAX_FRAME, MAX_OBJECT};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub kind: u8,
    pub transfer: Id,
    pub seq: u64,
    pub body: Vec<u8>,
}
impl Frame {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if self.body.len() > MAX_FRAME - 56 {
            return Err(Error::Limit);
        }
        if !matches!(self.kind, 1..=6 | 101..=105 | 127) {
            return Err(Error::Unsupported);
        }
        let mut b = Vec::with_capacity(56 + self.body.len());
        b.extend_from_slice(b"SCMOBJ01");
        b.extend_from_slice(&[self.kind, 0, 0, 0]);
        b.extend_from_slice(&self.transfer);
        b.extend_from_slice(&self.seq.to_le_bytes());
        b.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
        b.extend_from_slice(&self.body);
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() > MAX_FRAME {
            return Err(Error::Limit);
        }
        let mut r = Reader::new(b);
        if r.take(8)? != b"SCMOBJ01" {
            return Err(Error::Malformed);
        }
        let kind = r.u8()?;
        if !matches!(kind, 1..=6 | 101..=105 | 127) {
            return Err(Error::Unsupported);
        }
        if r.take(3)? != [0, 0, 0] {
            return Err(Error::Malformed);
        }
        let transfer = r.id()?;
        let seq = r.u64()?;
        let n = r.u32()? as usize;
        let body = r.take(n)?.to_vec();
        r.end()?;
        Ok(Self {
            kind,
            transfer,
            seq,
            body,
        })
    }
    pub fn reply(&self, kind: u8, body: Vec<u8>) -> Self {
        Self {
            kind,
            transfer: self.transfer,
            seq: self.seq,
            body,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Raw = 0,
    Zstd = 1,
    Dag = 2,
    SparseV1 = 3,
    ZstdPrefixV1 = 4,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub mode: Mode,
    pub len: u64,
    pub object: Id,
    pub manifest: Option<Manifest>,
    pub base: Option<Id>,
}
impl Offer {
    pub fn cold(bytes: &[u8], mode: Mode) -> Self {
        Self {
            mode,
            len: bytes.len() as u64,
            object: crate::object_id(bytes),
            manifest: None,
            base: None,
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut b = vec![self.mode as u8];
        b.extend_from_slice(&self.len.to_le_bytes());
        b.extend_from_slice(&self.object);
        let m = self
            .manifest
            .as_ref()
            .map(Manifest::encode)
            .unwrap_or_default();
        b.extend_from_slice(&(m.len() as u32).to_le_bytes());
        b.extend_from_slice(&m);
        b.push(u8::from(self.base.is_some()));
        if let Some(id) = self.base {
            b.extend_from_slice(&id);
        }
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let mode = match r.u8()? {
            0 => Mode::Raw,
            1 => Mode::Zstd,
            2 => Mode::Dag,
            3 => Mode::SparseV1,
            4 => Mode::ZstdPrefixV1,
            _ => return Err(Error::Unsupported),
        };
        let len = r.u64()?;
        let object = r.id()?;
        let n = r.u32()? as usize;
        if n > 1024 {
            return Err(Error::Limit);
        }
        let manifest = if n == 0 {
            None
        } else {
            Some(Manifest::decode(r.take(n)?)?)
        };
        let base = match r.u8()? {
            0 => None,
            1 => Some(r.id()?),
            _ => return Err(Error::Malformed),
        };
        r.end()?;
        let o = Self {
            mode,
            len,
            object,
            manifest,
            base,
        };
        o.validate()?;
        Ok(o)
    }
    fn validate(&self) -> Result<()> {
        if self.len > MAX_OBJECT {
            return Err(Error::Limit);
        }
        if self.mode == Mode::ZstdPrefixV1
            && self.len > crate::zstd_prefix::MAX_PREFIX_OBJECT as u64
        {
            return Err(Error::Limit);
        }
        match (self.mode, &self.manifest) {
            (Mode::Dag, Some(m)) if m.len == self.len && m.object == self.object => m.validate(),
            (Mode::Raw | Mode::Zstd, None) if self.base.is_none() => Ok(()),
            (Mode::SparseV1, None) if self.base.is_some() => Ok(()),
            (Mode::ZstdPrefixV1, None) if self.base.is_some() => Ok(()),
            _ => Err(Error::Malformed),
        }
    }
    pub fn id(&self) -> Result<Id> {
        Ok(hash(b"spiralcore.object.transfer.v1\0", &[&self.encode()?]))
    }
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Error::Limit)?;
        let b = self.bytes.get(self.pos..end).ok_or(Error::Malformed)?;
        self.pos = end;
        Ok(b)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| Error::Malformed)?,
        ))
    }
    pub fn id(&mut self) -> Result<Id> {
        self.take(32)?.try_into().map_err(|_| Error::Malformed)
    }
    pub fn end(&self) -> Result<()> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
    pub fn remaining(&mut self) -> &'a [u8] {
        let b = &self.bytes[self.pos..];
        self.pos = self.bytes.len();
        b
    }
}
