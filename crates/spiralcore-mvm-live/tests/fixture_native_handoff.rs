//! Local custody controls for the optional signed-export handoff.
#![cfg(all(
    feature = "canonical-host",
    target_os = "linux",
    target_arch = "x86_64"
))]

#[path = "support/native_handoff.rs"]
mod native_handoff;

use anyhow::Result;
use serde_json::json;
use std::{
    fs, io,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir};

fn private_dir(parent: &Path) -> io::Result<TempDir> {
    Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(parent)
}

struct Case {
    owner: TempDir,
    work: TempDir,
    root: PathBuf,
    ready: PathBuf,
    digest: String,
}

impl Case {
    fn new() -> Result<Self> {
        let owner = private_dir(&std::env::temp_dir().canonicalize()?)?;
        let work = private_dir(owner.path())?;
        let root = work.path().to_owned();
        let packet = root.join("packet");
        fs::create_dir(&packet)?;
        let archive = b"handoff control\0";
        fs::write(packet.join("workload.mvmpkg"), archive)?;
        let digest = mvm_contract::hash::sha256_hex(archive);
        let ready = owner.path().join("native-ready");
        Ok(Self {
            owner,
            work,
            root,
            ready,
            digest,
        })
    }
}

fn absent(path: &Path) {
    assert_eq!(
        fs::symlink_metadata(path)
            .expect_err("unexpected surviving entry")
            .kind(),
        io::ErrorKind::NotFound
    );
}

#[test]
fn fresh_private_ready_identifies_original_retained_root() -> Result<()> {
    let c = Case::new()?;
    let original = fs::metadata(&c.root)?;
    native_handoff::finish(c.work, Some(&c.ready), &c.digest)?;
    let entry = fs::symlink_metadata(&c.ready).expect("missing private ready record");
    assert!(entry.is_file());
    assert_eq!(
        (entry.mode() & 0o7777, entry.nlink(), entry.uid()),
        (0o600, 1, original.uid())
    );
    assert!(entry.len() <= 8192);
    let record: serde_json::Value = serde_json::from_slice(&fs::read(&c.ready)?)?;
    assert_eq!(
        record,
        json!({"root":c.root,"dev":original.dev(),"ino":original.ino(),
        "uid":original.uid(),"archive_sha256":c.digest})
    );
    let after = fs::symlink_metadata(&c.root)?;
    assert_eq!(
        (after.dev(), after.ino(), after.uid()),
        (original.dev(), original.ino(), original.uid())
    );
    assert_eq!(
        fs::read(c.root.join("packet/workload.mvmpkg"))?,
        b"handoff control\0"
    );
    Ok(())
}

#[test]
fn absent_option_releases_original_tempdir() -> Result<()> {
    let c = Case::new()?;
    native_handoff::finish(c.work, None, &c.digest)?;
    absent(&c.root);
    absent(&c.ready);
    Ok(())
}

#[test]
fn occupied_ready_preserves_original_and_releases_tempdir() -> Result<()> {
    for dangling in [false, true] {
        let c = Case::new()?;
        if dangling {
            symlink("uncreated", &c.ready)?;
        } else {
            fs::write(&c.ready, b"prior owner")?;
        }
        let original = fs::symlink_metadata(&c.ready)?;
        assert!(
            native_handoff::finish(c.work, Some(&c.ready), &c.digest).is_err(),
            "occupied ready accepted"
        );
        let after = fs::symlink_metadata(&c.ready)?;
        assert_eq!((after.dev(), after.ino()), (original.dev(), original.ino()));
        if dangling {
            assert_eq!(fs::read_link(&c.ready)?, Path::new("uncreated"));
            absent(&c.owner.path().join("uncreated"));
        } else {
            assert_eq!(fs::read(&c.ready)?, b"prior owner");
        }
        absent(&c.root);
    }
    Ok(())
}

#[test]
fn unsafe_ready_parent_refuses_without_retention() -> Result<()> {
    let c = Case::new()?;
    let parent = private_dir(c.owner.path())?;
    fs::set_permissions(parent.path(), fs::Permissions::from_mode(0o755))?;
    let ready = parent.path().join("ready");
    assert!(
        native_handoff::finish(c.work, Some(&ready), &c.digest).is_err(),
        "nonprivate ready parent accepted"
    );
    absent(&ready);
    absent(&c.root);
    Ok(())
}

#[test]
fn aliased_ready_parent_refuses_without_retention() -> Result<()> {
    let c = Case::new()?;
    let actual = private_dir(c.owner.path())?;
    let alias = c.owner.path().join("alias");
    symlink(actual.path(), &alias)?;
    assert!(
        native_handoff::finish(c.work, Some(&alias.join("ready")), &c.digest).is_err(),
        "symlinked ready parent accepted"
    );
    absent(&actual.path().join("ready"));
    absent(&c.root);
    Ok(())
}
