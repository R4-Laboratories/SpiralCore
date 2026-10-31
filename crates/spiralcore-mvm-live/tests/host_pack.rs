#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeMap;

use spiralcore_mvm_live::host::{create_disk, initramfs};

struct Entry {
    mode: u32,
    data: Vec<u8>,
}

fn entries(archive: &[u8]) -> BTreeMap<String, Entry> {
    let mut entries = BTreeMap::new();
    let mut offset = 0;
    loop {
        assert_eq!(&archive[offset..offset + 6], b"070701");
        let field = |number: usize| {
            let begin = offset + 6 + number * 8;
            u32::from_str_radix(
                std::str::from_utf8(&archive[begin..begin + 8]).unwrap(),
                16,
            )
            .unwrap()
        };
        let mode = field(1);
        assert_eq!((field(2), field(3)), (0, 0), "root-owned entries");
        let data_len = field(6) as usize;
        let name_len = field(11) as usize;
        let begin = offset + 110;
        assert_eq!(archive[begin + name_len - 1], 0);
        let name = std::str::from_utf8(&archive[begin..begin + name_len - 1]).unwrap();
        let data_begin = (begin + name_len).next_multiple_of(4);
        offset = (data_begin + data_len).next_multiple_of(4);
        if name == "TRAILER!!!" {
            assert!(archive[offset..].iter().all(|byte| *byte == 0));
            break;
        }
        assert!(!name.starts_with('/'));
        assert!(!name.split('/').any(|part| part == ".."));
        assert!(
            entries
                .insert(
                    name.to_owned(),
                    Entry {
                        mode,
                        data: archive[data_begin..data_begin + data_len].to_vec(),
                    },
                )
                .is_none(),
            "duplicate archive member"
        );
    }
    entries
}

fn guest_header() -> Vec<u8> {
    let mut elf = vec![0; 64];
    elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    elf[16..18].copy_from_slice(&2_u16.to_le_bytes());
    elf[18..20].copy_from_slice(&183_u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    elf
}

#[test]
fn packs_exact_guest_bytes_and_private_bootstrap_with_only_fixed_paths() {
    let guest = guest_header();
    let bootstrap = br#"{"synthetic_fixture":"private-in-memory-only"}"#;
    let archive = initramfs(&guest, bootstrap).expect("bounded initramfs");
    let decoded = entries(&archive);
    assert_eq!(decoded.len(), 9);
    assert_eq!(decoded["init"].mode, 0o100755);
    assert_eq!(decoded["init"].data, guest);
    assert_eq!(decoded["bootstrap.json"].mode, 0o100600);
    assert_eq!(decoded["bootstrap.json"].data, bootstrap);
    for directory in [".", "proc", "sys", "dev", "tmp", "run", "data"] {
        assert_eq!(decoded[directory].mode, 0o040755);
        assert!(decoded[directory].data.is_empty());
    }
    assert_eq!(archive, initramfs(&guest, bootstrap).unwrap());
}

#[test]
fn refuses_oversized_or_malformed_bootstrap_and_guest_inputs() {
    let guest = guest_header();
    for bootstrap in [&b""[..], &b"not json"[..], &b"null"[..], &b"[]"[..]] {
        assert!(initramfs(&guest, bootstrap).is_err());
    }
    assert!(initramfs(&guest, &vec![b' '; 64 * 1024 + 1]).is_err());
    assert!(initramfs(&vec![0; 32 * 1024 * 1024 + 1], b"{}").is_err());
    assert!(initramfs(&[], b"{}").is_err());
    assert!(initramfs(b"not an ELF executable", b"{}").is_err());
    let mut wrong_arch = guest;
    wrong_arch[18..20].copy_from_slice(&62_u16.to_le_bytes());
    assert!(initramfs(&wrong_arch, b"{}").is_err());
}

#[test]
fn creates_private_sparse_ext4_and_never_overwrites_an_existing_path() {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::MetadataExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cache.ext4");
    create_disk(&path).expect("fresh disk");
    let mut disk = std::fs::File::open(&path).unwrap();
    let metadata = disk.metadata().unwrap();
    assert_eq!(metadata.len(), 256 * 1024 * 1024);
    assert_eq!(metadata.mode() & 0o777, 0o600);
    assert!(metadata.blocks() * 512 < metadata.len() / 8);
    disk.seek(SeekFrom::Start(1024)).unwrap();
    let mut superblock = [0; 1024];
    disk.read_exact(&mut superblock).unwrap();
    assert_eq!(&superblock[56..58], &0xef53_u16.to_le_bytes());
    assert_eq!(&superblock[0x78..0x84], b"spiral-cache");
    assert!(create_disk(&path).is_err());
    disk.seek(SeekFrom::Start(1024)).unwrap();
    let mut after = [0; 1024];
    disk.read_exact(&mut after).unwrap();
    assert_eq!(superblock, after);

    let sentinel = directory.path().join("existing-file");
    std::fs::write(&sentinel, b"preserve exact existing bytes").unwrap();
    assert!(create_disk(&sentinel).is_err());
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"preserve exact existing bytes"
    );
}
