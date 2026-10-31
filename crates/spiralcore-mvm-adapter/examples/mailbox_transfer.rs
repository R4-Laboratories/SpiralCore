#[cfg(test)]
mod tests {
    #[test]
    fn mixed_session_records_exact_reopened_objects_and_canonical_bytes() {
        let root = std::env::temp_dir().join(format!("spiral-mvm-example-{}", std::process::id()));
        assert!(!root.exists());
        let mut rows = Vec::new();
        super::run_session(&root, "warm", 4096, 0, &mut rows).unwrap();
        let rows: Vec<serde_json::Value> = String::from_utf8(rows).unwrap()
            .lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        assert_eq!(rows.iter().filter(|r| r["event"] == "operation").count(), 7);
        assert_eq!(rows.iter().filter(|r| r["event"] == "verified" && r["exact"] == true).count(), 7);
        for row in rows.iter().filter(|r| r["event"] == "operation") {
            assert!(row["envelope_bytes"].as_u64().unwrap() > row["frame_bytes"].as_u64().unwrap());
            assert_eq!(row["envelope_bytes"], row["received_envelope_bytes"]);
        }
        assert!(!root.exists(), "only verified owned stores should be removed");
    }
}
