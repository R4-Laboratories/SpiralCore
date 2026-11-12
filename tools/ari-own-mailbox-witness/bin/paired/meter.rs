use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
#[path = "../guest_stream.rs"]
mod stream;
pub struct OriginalMeter {
    file: File,
}
impl OriginalMeter {
    pub fn open(name: &str) -> Result<Self> {
        Self::open_named(name, stream::NAME)
    }
    fn open_named(name: &str, filename: &str) -> Result<Self> {
        let base = mvm_core::config::vm_state_dir(name);
        let path = base.join(filename);
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)?;
        let m = file.metadata()?;
        let path_meta = std::fs::symlink_metadata(&path)?;
        ensure!(
            m.is_file()
                && m.nlink() == 1
                && m.mode() & 0o777 == 0o600
                && path_meta.dev() == m.dev()
                && path_meta.ino() == m.ino()
                && m.uid() == std::fs::metadata(&base)?.uid(),
            "unverified original helper meter"
        );
        Ok(Self { file })
    }
    pub fn finish(self, path: &Path) -> Result<Value> {
        let mut bytes = String::new();
        self.file.take(32_769).read_to_string(&mut bytes)?;
        ensure!(bytes.len() <= 32_768, "oversized original helper meter");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(bytes.as_bytes())?;
        file.sync_all()?;
        Ok(match stream::completed(&bytes) {
            Some((n, read, write, whole)) => {
                json!({"completed_exchanges":n,"guest_to_host_read_bytes":read,"host_to_guest_write_accepted_bytes":write,"capture":if whole{"closed_session"}else{"prefix_only"}})
            }
            None => {
                json!({"capture":"incomplete","completed_exchanges":null,"guest_to_host_read_bytes":null,"host_to_guest_write_accepted_bytes":null})
            }
        })
    }
}

#[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
pub(super) mod selected {
    use super::*;
    use anyhow::Context;
    use std::os::unix::fs::FileExt;
    const SEALED: &str = "mvm/owned-mailbox/selected-object/sealed-stream/read-write/v1";
    const CHANNELS: [(&str, &str, &str); 3] = [
        (
            "guest_5257",
            "object-guest-stream-v1.counts",
            stream::SELECTED_GUEST,
        ),
        ("sealed_in", "object-sealed-in-v1.counts", SEALED),
        ("sealed_out", "object-sealed-out-v1.counts", SEALED),
    ];
    pub struct Meters(Vec<OriginalMeter>);
    impl Meters {
        pub fn open(name: &str) -> Result<Self> {
            CHANNELS
                .iter()
                .map(|(_, filename, _)| OriginalMeter::open_named(name, filename))
                .collect::<Result<Vec<_>>>()
                .map(Self)
        }
        pub fn snapshot(&self, capture: Option<(&Path, &str)>) -> Result<Value> {
            let mut result = serde_json::Map::new();
            for (original, (key, _, scope)) in self.0.iter().zip(CHANNELS) {
                let len = usize::try_from(original.file.metadata()?.len())?;
                ensure!(
                    len <= if scope == stream::SELECTED_GUEST {
                        65536
                    } else {
                        32768
                    },
                    "original stream meter exceeds bound"
                );
                let mut bytes = vec![0; len];
                original.file.read_exact_at(&mut bytes, 0)?;
                if let Some((home, name)) = capture {
                    let mut file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .mode(0o600)
                        .open(home.join(format!("original-{name}-{key}.counts")))?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                }
                let text = std::str::from_utf8(&bytes).context("original meter is not UTF-8")?;
                let value = match stream::completed_for(text, scope) {
                    Some((n, read, write, re, we, finished)) => {
                        json!({"capture":if finished{"closed_session"}else{"complete_prefix"},"turns":n,"read":read,"write":write,"read_errors":re,"write_errors":we})
                    }
                    None if capture.is_none() && text.strip_suffix('\n') == Some(scope) => {
                        json!({"capture":"empty_prefix","turns":0,"read":0,"write":0,"read_errors":0,"write_errors":0})
                    }
                    None => json!({"capture":"incomplete","read":null,"write":null}),
                };
                result.insert(key.into(), value);
            }
            Ok(Value::Object(result))
        }
    }
}
