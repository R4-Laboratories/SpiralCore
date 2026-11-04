// Compiler-only error adapter. The exact backend below is never executed here.
#[derive(Debug)]
pub enum Error { Storage, Unsupported, Limit }
pub type Result<T> = std::result::Result<T, Error>;
impl From<std::io::Error> for Error { fn from(_: std::io::Error) -> Self { Self::Storage } }
pub mod endpoint {
    pub mod storage {
        #[path = "/var/tmp/spiralcore-windows-storage-20261103/source/crates/spiralcore-transfer/src/endpoint/storage/windows.rs"]
        mod windows;
        pub fn signatures() {
            use std::{fs::File, path::Path};
            let _: fn(&Path) -> crate::Result<()> = windows::private_file;
            let _: fn(&Path) -> crate::Result<()> = windows::private_dir;
            let _: fn(&Path) -> crate::Result<File> = windows::open_read;
            let _: fn(&Path) -> crate::Result<File> = windows::create_new_file;
            let _: fn(&Path) -> crate::Result<File> = windows::open_owner;
            let _: fn(&Path) -> crate::Result<()> = windows::sync_existing;
            let _: fn(&Path) -> crate::Result<()> = windows::remove_stage;
            let _: fn(&Path, &Path, &Path) -> crate::Result<()> = windows::publish;
        }
    }
}
