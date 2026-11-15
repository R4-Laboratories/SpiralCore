use super::*;
use std::{collections::BTreeSet, fs, os::unix::ffi::OsStrExt};

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sc-directory-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn open(&self) -> OwnedFd {
        fs::File::open(&self.0).unwrap().into()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn native_refills_preserve_every_name_and_independent_offsets() {
    let fixture = Fixture::new();
    let expected: BTreeSet<_> = (0..320)
        .map(|i| {
            let name = format!("{i:04}-é-{}", "x".repeat(190));
            fs::File::create(fixture.0.join(&name)).unwrap();
            name.into_bytes()
        })
        .collect();
    assert!(expected.iter().map(|n| n.len()).sum::<usize>() > crate::BUFFER_BYTES * 3);
    let mut buffer = vec![0; BUFFER_WORDS];
    let mut first = BTreeSet::new();
    assert_eq!(
        for_each(fixture.open(), &mut buffer, |name| {
            assert!(first.insert(name.as_bytes().to_vec()));
            ControlFlow::Continue(())
        })
        .unwrap(),
        ControlFlow::Continue(())
    );
    assert_eq!(first, expected);
    let mut stopped = 0;
    assert_eq!(
        for_each(fixture.open(), &mut buffer, |_| {
            stopped += 1;
            ControlFlow::Break(())
        })
        .unwrap(),
        ControlFlow::Break(())
    );
    assert_eq!(stopped, 1);
    let mut second = BTreeSet::new();
    assert_eq!(
        for_each(fixture.open(), &mut buffer, |name| {
            assert!(second.insert(name.as_bytes().to_vec()));
            ControlFlow::Continue(())
        })
        .unwrap(),
        ControlFlow::Continue(())
    );
    assert_eq!(second, expected);
}

#[test]
fn native_refuses_wrong_buffer_and_regular_file_without_callbacks() {
    let fixture = Fixture::new();
    assert_eq!(
        for_each(fixture.open(), &mut [0; 1], |_| panic!(
            "invalid buffer callback"
        ))
        .unwrap_err()
        .kind(),
        io::ErrorKind::InvalidInput
    );
    let file = fs::File::create(fixture.0.join("file")).unwrap();
    let mut buffer = vec![0; BUFFER_WORDS];
    assert!(for_each(file.into(), &mut buffer, |_| panic!(
        "regular file callback"
    ))
    .is_err());
}
