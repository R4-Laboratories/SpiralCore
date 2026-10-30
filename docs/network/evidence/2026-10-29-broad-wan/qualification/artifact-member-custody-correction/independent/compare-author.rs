use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
fn read(path: &Path) -> String {
    let t = fs::read_to_string(path).unwrap();
    assert!(t.len() <= 1024 * 1024);
    t
}
fn main() {
    let root = std::env::args().nth(1).expect("correction root");
    let root = Path::new(&root);
    let review_text = read(&root.join("independent/independent-review.json"));
    let review: Value = serde_json::from_str(&review_text).unwrap();
    assert_eq!(review["status"], "passed");
    assert_eq!(review["archives"], 11);
    assert_eq!(review["members_compared"], 613);
    let mut reports = Vec::new();
    let mut members = 0usize;
    for r in review["records"].as_array().unwrap() {
        let dir = Path::new(r["directory"].as_str().unwrap());
        let shard = dir.file_name().unwrap().to_str().unwrap();
        let case = dir.parent().unwrap().file_name().unwrap().to_str().unwrap();
        let label = if shard.contains("preflight-failure") {
            "cold_random-preflight-attempt-1".to_owned()
        } else {
            format!("{case}-{shard}")
        };
        let author_path = root.join("rechecks").join(&label);
        let author_text = read(&author_path.join("recheck.json"));
        let a: Value = serde_json::from_str(&author_text).unwrap();
        assert_eq!(a["schema"], "spiralcore.artifact-member-recheck.v1");
        assert_eq!(a["status"], "passed");
        assert_eq!(a["archive_sha256"], r["archive_sha256"]);
        assert_eq!(a["archive_bytes"], r["archive_bytes"]);
        assert_eq!(a["remote_directory"], dir.join("remote").to_str().unwrap());
        for field in ["expected_members", "visited_members", "manifest_lines"] {
            assert_eq!(a[field], r["comparison"]["visited_members"]);
        }
        assert_eq!(
            a["uncompressed_bytes_compared"],
            r["comparison"]["uncompressed_bytes_compared"]
        );
        let expected: String = r["comparison"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                format!(
                    "{}  {}\n",
                    m["sha256"].as_str().unwrap(),
                    m["name"].as_str().unwrap()
                )
            })
            .collect();
        let actual = read(&author_path.join("remote-files.sha256"));
        assert_eq!(expected, actual);
        let sha = format!("{:x}", Sha256::digest(actual.as_bytes()));
        assert_eq!(a["manifest_sha256"], sha);
        assert_eq!(r["corrected_manifest_sha256"], sha);
        let count = actual.lines().count();
        assert!(count > 0);
        members = members.checked_add(count).unwrap();
        reports.push(json!({"label":label,"members":count,"manifest_sha256":sha,"author_receipt_sha256":format!("{:x}",Sha256::digest(author_text.as_bytes())),"all_member_lines_equal":true}));
    }
    assert_eq!(reports.len(), 11);
    assert_eq!(members, 613);
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"spiralcore.independent-member-author-crosscheck.v1","status":"passed","archives":reports.len(),"members":members,"independent_review_sha256":format!("{:x}",Sha256::digest(review_text.as_bytes())),"records":reports})).unwrap());
}
