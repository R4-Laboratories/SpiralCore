// Additional destructive-cleanup fence, never a replacement for SDK authority.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs::{File, OpenOptions}, io::Read, os::unix::fs::{MetadataExt, OpenOptionsExt}, path::{Path, PathBuf}};
const MAX_DEFINITION_BYTES: u64 = 1024 * 1024;
pub struct DefinitionGuard { file: File, path: PathBuf, digest: [u8;32] }
impl DefinitionGuard {
    fn open(path: &Path) -> Result<(File,[u8;32])> {
        let mut file=OpenOptions::new().read(true).custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32).open(path)?;
        let before=file.metadata()?;
        ensure!(before.is_file() && before.uid()==rustix::process::geteuid().as_raw() && before.mode() & 0o777 == 0o600 && before.nlink()==1 && before.len()<=MAX_DEFINITION_BYTES, "original definition must be privately owned bounded regular file");
        let mut bytes=Vec::new();(&mut file).take(MAX_DEFINITION_BYTES+1).read_to_end(&mut bytes)?;
        let after=file.metadata()?;
        ensure!(bytes.len() as u64 == before.len() && same(&before,&after) && before.mtime()==after.mtime() && before.mtime_nsec()==after.mtime_nsec() && before.ctime()==after.ctime() && before.ctime_nsec()==after.ctime_nsec(), "definition changed during capture");
        Ok((file,Sha256::digest(&bytes).into()))
    }
    pub fn capture(path: &Path) -> Result<Self> {
        let (file,digest)=Self::open(path)?;Ok(Self{file,path:path.into(),digest})
    }
    pub fn verify(&self) -> Result<()> {
        let (current,digest)=Self::open(&self.path)?;
        ensure!(same(&self.file.metadata()?,&current.metadata()?) && digest==self.digest,"original definition changed or was replaced; retain state");Ok(())
    }
}
fn same(a:&std::fs::Metadata,b:&std::fs::Metadata)->bool { a.dev()==b.dev() && a.ino()==b.ino() && a.len()==b.len() }
