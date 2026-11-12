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
        let base = mvm_core::config::vm_state_dir(name);
        let path = base.join(stream::NAME);
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
