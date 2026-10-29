//! Non-timed cross-backend identity export. The measured probe source is unchanged.
use sha2::{Digest, Sha256};
use spiralcore_receiver_probe::{fixture, Result};
use spiralcore_transfer::{dag::Graph, hex, object_id};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

fn random_bytes(len: usize) -> Vec<u8> {
    // Same deterministic sequence used by the frozen direct_dag canonical oracle.
    let mut state = 0x9e3779b97f4a7c15u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn cases() -> Vec<(String, Vec<u8>)> {
    let mut out = vec![
        ("empty".into(), vec![]),
        ("zero1".into(), vec![0]),
        ("zero2".into(), vec![0, 0]),
        ("a".into(), b"a".to_vec()),
        ("a-zero1".into(), b"a\0".to_vec()),
        ("a-zero2".into(), b"a\0\0".to_vec()),
        ("all-bytes".into(), (0..=255u8).collect()),
    ];
    for exponent in 0..=7 {
        let boundary = 32 * 3usize.pow(exponent);
        for len in [boundary - 1, boundary, boundary + 1] {
            out.push((format!("random-{len}"), random_bytes(len)));
        }
    }
    out.push(("zero32768".into(), vec![0; 32768]));
    out.push((
        "period64".into(),
        (0..32768).map(|i| (i % 64) as u8).collect(),
    ));
    out.push((
        "blocks96".into(),
        (0..32768).map(|i| ((i / 96) % 251) as u8).collect(),
    ));
    let (base, target) = fixture(42);
    out.push(("receiver-base".into(), base));
    out.push(("receiver-target".into(), target));
    out
}

fn create(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .ok_or("one new absolute output directory required")?;
    if args.next().is_some() || !Path::new(&dir).is_absolute() {
        return Err("one new absolute output directory required".into());
    }
    let dir = Path::new(&dir);
    fs::create_dir(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    let mut rows = BufWriter::new(create(&dir.join("identities.jsonl"))?);
    for (i, (name, bytes)) in cases().into_iter().enumerate() {
        let graph = Graph::fold(&bytes, 262144)?;
        if graph.rebuild()? != bytes || graph.manifest.object != object_id(&bytes) {
            return Err("graph identity/rebuild mismatch".into());
        }
        let file_name = format!("case-{i:02}.nodes");
        let mut file = BufWriter::new(create(&dir.join(&file_name))?);
        let mut fingerprint = Sha256::new();
        // Exact canonical BTreeMap order, length-framed complete node encodings.
        // Both the complete files and their fingerprints are compared across builds.
        for (id, node) in &graph.nodes {
            if node.id() != *id {
                return Err("node identity mismatch".into());
            }
            let encoded = node.encode();
            let len = u32::try_from(encoded.len())?.to_le_bytes();
            file.write_all(&len)?;
            file.write_all(&encoded)?;
            fingerprint.update(len);
            fingerprint.update(&encoded);
        }
        file.flush()?;
        file.get_ref().sync_all()?;
        let row = serde_json::json!({
            "case":name,"bytes":bytes.len(),"object_id":hex(&graph.manifest.object),
            "manifest_bytes":graph.manifest.encode(),"manifest_id":hex(&graph.manifest.id()),
            "node_count":graph.nodes.len(),"nodes_file":file_name,
            "node_serialization_sha256":hex(&fingerprint.finalize().into()),
            "input_sha256":hex(&Sha256::digest(&bytes).into()),"exact_rebuild":true
        });
        serde_json::to_writer(&mut rows, &row)?;
        rows.write_all(b"\n")?;
    }
    rows.flush()?;
    rows.get_ref().sync_all()?;
    File::open(dir)?.sync_all()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("identity qualification: {e}");
        std::process::exit(1);
    }
}
