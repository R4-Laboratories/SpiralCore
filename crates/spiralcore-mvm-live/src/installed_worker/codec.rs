use super::{ControlError, MAX_ROOT, Result, WorkerBinding, io_util};
use std::{
    ffi::OsString,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        net::UnixStream,
    },
    path::PathBuf,
    time::Instant,
};

const MAX_PAYLOAD: usize = 1020;
pub(super) enum Record {
    Hello([u8; 32]),
    Binding([u8; 32], WorkerBinding),
    Ready([u8; 32]),
    BootChallenge([u8; 32]),
    Go([u8; 32], WorkerBinding),
}
pub(super) fn write_record(
    stream: &mut UnixStream,
    deadline: Instant,
    record: &Record,
) -> Result<()> {
    let (kind, nonce, binding) = match record {
        Record::Hello(n) => (1, n, None),
        Record::Binding(n, b) => (2, n, Some(b)),
        Record::Ready(n) => (3, n, None),
        Record::BootChallenge(n) => (4, n, None),
        Record::Go(n, b) => (5, n, Some(b)),
    };
    let mut bytes = Vec::with_capacity(MAX_PAYLOAD);
    bytes.extend_from_slice(&[1, kind]);
    bytes.extend_from_slice(nonce);
    if let Some(binding) = binding {
        binding.validate(binding.role)?;
        let path = binding.root.as_os_str().as_bytes();
        bytes.extend_from_slice(&(path.len() as u16).to_be_bytes());
        bytes.extend_from_slice(path);
        bytes.extend_from_slice(&binding.installation);
        bytes.extend_from_slice(&binding.profile);
        bytes.extend_from_slice(&binding.generation.to_be_bytes());
        bytes.extend_from_slice(&[binding.role, binding.slot]);
        bytes.extend_from_slice(&binding.serial.to_be_bytes());
    }
    if bytes.len() > MAX_PAYLOAD {
        return Err(ControlError::Protocol);
    }
    io_util::write_all(stream, &(bytes.len() as u32).to_be_bytes(), deadline)?;
    io_util::write_all(stream, &bytes, deadline)
}
pub(super) fn read_record(stream: &mut UnixStream, deadline: Instant) -> Result<Record> {
    let mut prefix = [0; 4];
    io_util::read_exact(stream, &mut prefix, deadline)?;
    let size = u32::from_be_bytes(prefix) as usize;
    if !(34..=MAX_PAYLOAD).contains(&size) {
        return Err(ControlError::Protocol);
    }
    let mut bytes = vec![0; size];
    io_util::read_exact(stream, &mut bytes, deadline)?;
    if bytes[0] != 1 {
        return Err(ControlError::Protocol);
    }
    let kind = bytes[1];
    let nonce: [u8; 32] = bytes[2..34]
        .try_into()
        .map_err(|_| ControlError::Protocol)?;
    if matches!(kind, 1 | 3 | 4) && size == 34 {
        return Ok(match kind {
            1 => Record::Hello(nonce),
            3 => Record::Ready(nonce),
            _ => Record::BootChallenge(nonce),
        });
    }
    if !matches!(kind, 2 | 5) || size < 36 {
        return Err(ControlError::Protocol);
    }
    let path_len = u16::from_be_bytes(
        bytes[34..36]
            .try_into()
            .map_err(|_| ControlError::Protocol)?,
    ) as usize;
    if path_len == 0 || path_len > MAX_ROOT || size != 118 + path_len {
        return Err(ControlError::Protocol);
    }
    let mut at = 36 + path_len;
    let root = PathBuf::from(OsString::from_vec(bytes[36..at].to_vec()));
    let installation = bytes[at..at + 32]
        .try_into()
        .map_err(|_| ControlError::Protocol)?;
    at += 32;
    let profile = bytes[at..at + 32]
        .try_into()
        .map_err(|_| ControlError::Protocol)?;
    at += 32;
    let generation = u64::from_be_bytes(
        bytes[at..at + 8]
            .try_into()
            .map_err(|_| ControlError::Protocol)?,
    );
    at += 8;
    let role = bytes[at];
    let slot = bytes[at + 1];
    at += 2;
    let serial = u64::from_be_bytes(
        bytes[at..at + 8]
            .try_into()
            .map_err(|_| ControlError::Protocol)?,
    );
    let binding = WorkerBinding {
        root,
        installation,
        profile,
        generation,
        role,
        slot,
        serial,
    };
    binding.validate(role)?;
    Ok(if kind == 2 {
        Record::Binding(nonce, binding)
    } else {
        Record::Go(nonce, binding)
    })
}
