//! Observe the original store without opening a second codec or granting custody.
use super::*;
use mvm_core::host_lifetime::EndpointStoreLocator;
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

pub(super) struct Store {
    original: File,
    lock: File,
    path: PathBuf,
    pub observation: serde_json::Value,
}

fn opened(path: &std::path::Path, directory: bool) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(
            libc::O_NOFOLLOW | libc::O_NONBLOCK | if directory { libc::O_DIRECTORY } else { 0 },
        )
        .open(path)?;
    let meta = file.metadata()?;
    ensure!(meta.is_dir() == directory && meta.mode() & 0o077 == 0);
    mvm_core::private_fs::require_current_owner(&meta)?;
    if !directory {
        ensure!(meta.is_file() && meta.nlink() == 1);
    }
    Ok(file)
}

pub(super) fn observe(
    locator: &EndpointStoreLocator,
    flow: [u8; 32],
    namespace: [u8; 32],
) -> Result<Store> {
    let original = opened(&locator.path, true)?;
    let identity = original.metadata()?;
    ensure!((identity.dev(), identity.ino()) == (locator.device, locator.inode));
    let directory = locator.path.join(format!("flow-{}", hex::encode(flow)));
    let held = opened(&directory, true)?;
    let expected: BTreeSet<_> = [
        ".owner".to_owned(),
        ".spiralcore-profile-v1".into(),
        hex::encode(namespace),
    ]
    .into();
    let names: BTreeSet<_> = fs::read_dir(&directory)?
        .map(|e| {
            e?.file_name()
                .into_string()
                .map_err(|_| anyhow!("non-UTF8 store observation"))
        })
        .collect::<Result<_>>()?;
    ensure!(
        names == expected,
        "original live namespace/profile shape differs"
    );
    let lock = opened(&directory.join(".owner"), false)?;
    ensure!(
        matches!(lock.try_lock(), Err(std::fs::TryLockError::WouldBlock)),
        "original live writer lock not held"
    );
    let mut profile = Vec::new();
    opened(&directory.join(".spiralcore-profile-v1"), false)?
        .take(64)
        .read_to_end(&mut profile)?;
    ensure!(profile == b"SpiralCore private object cache v1\n");
    let lane = directory.join(hex::encode(namespace));
    let _lane = opened(&lane, true)?;
    let expected: BTreeSet<_> = (0..BLOCKS)
        .map(|i| {
            format!(
                "{}.object",
                hex::encode(spiralcore_transfer::object_id(&peer::block(i)))
            )
        })
        .collect();
    let names: BTreeSet<_> = fs::read_dir(&lane)?
        .map(|e| {
            e?.file_name()
                .into_string()
                .map_err(|_| anyhow!("non-UTF8 publication"))
        })
        .collect::<Result<_>>()?;
    ensure!(
        names == expected,
        "original selected store does not retain all distinct publications"
    );
    let mut records = Vec::with_capacity(BLOCKS);
    for i in 0..BLOCKS {
        let body = peer::block(i);
        let name = format!(
            "{}.object",
            hex::encode(spiralcore_transfer::object_id(&body))
        );
        let file = opened(&lane.join(&name), false)?;
        let mut bytes = Vec::new();
        file.take((BLOCK_BYTES + 4096) as u64)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() < BLOCK_BYTES + 4096 && bytes.ends_with(&body),
            "stored payload tail differs"
        );
        // Protocol readback already performed canonical verification. This is
        // an independent bounded record/tail observation, not another decoder.
        records.push(
            serde_json::json!({"name":name,"record_bytes":bytes.len(),"payload_bytes":body.len()}),
        );
    }
    ensure!(
        (held.metadata()?.dev(), held.metadata()?.ino())
            == (
                fs::symlink_metadata(&directory)?.dev(),
                fs::symlink_metadata(&directory)?.ino()
            )
    );
    Ok(Store {
        original,
        lock,
        path: locator.path.clone(),
        observation: serde_json::json!({
            "original_device":identity.dev(),"original_inode":identity.ino(),
            "flow":hex::encode(flow),"namespace":hex::encode(namespace),
            "distinct_publications":records.len(),"records":records,"writer_lock":"held at populated barrier",
            "scope":"bounded controller observation; public codec verified every network readback"
        }),
    })
}

impl Store {
    pub(super) fn check_retired(&self) -> Result<()> {
        ensure!(
            fs::symlink_metadata(&self.path)
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        );
        ensure!(
            self.original.metadata()?.nlink() == 0,
            "original disposable store remains linked"
        );
        self.lock
            .try_lock()
            .context("original writer lock survived child retirement")?;
        self.lock.unlock()?;
        Ok(())
    }
}
