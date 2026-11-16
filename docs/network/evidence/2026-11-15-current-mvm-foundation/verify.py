#!/usr/bin/env python3
"""Verify compact frozen-source evidence; never execute archived programs."""
from pathlib import Path
import hashlib
import json
import zipfile

root = Path(__file__).resolve().parent
index = json.loads((root / "record-index.json").read_text())
archive = root / "records.zip"
assert archive.stat().st_size == index["archive_bytes"]
assert hashlib.sha256(archive.read_bytes()).hexdigest() == index["archive_sha256"]
with zipfile.ZipFile(archive) as records:
    rows = index["members"]
    assert len(rows) == 284
    assert len({row["name"] for row in rows}) == len(rows)
    assert set(records.namelist()) == {row["name"] for row in rows}
    for row in rows:
        data = records.read(row["name"])
        assert len(data) == row["bytes"], row["name"]
        assert hashlib.sha256(data).hexdigest() == row["sha256"], row["name"]
    prefix = "privacy-mvm-full1-"
    outcome = json.loads(records.read(prefix + "outcome.json"))
    assert set(outcome) == {"original_exit", "source_hash_exit", "file_enumeration_exit",
                            "file_set_comparison_exit", "final_exit"}
    assert all(value == 0 for value in outcome.values())
    frozen = records.read(prefix + "frozen-inputs.sha256")
    assert len(frozen.splitlines()) == 4746
    assert frozen == records.read(prefix + "file-set-before.sha256")
    assert frozen == records.read(prefix + "file-set-after.sha256")
    for suffix in ("inputs-before.txt", "inputs-after.txt"):
        lines = records.read(prefix + suffix).decode().splitlines()
        assert len(lines) == 4746 and all(line.endswith(": OK") for line in lines)
    artifacts = json.loads(records.read(prefix + "executables-before.json"))
    assert len(artifacts) == 285 and all(row["fresh"] is False for row in artifacts)
    executed = json.loads(records.read(prefix + "runtime-executable-set.json"))
    assert executed == {"expected": 285, "running_lines": 285,
                        "unique_executed": 285, "exact_match": True}
    hashes = records.read(prefix + "executables-after.txt").decode().splitlines()
    assert len(hashes) == 285 and all(line.endswith(": OK") for line in hashes)
    counts = json.loads(records.read(prefix + "test-counts.json"))
    assert counts == json.loads((root / "test-counts.json").read_text())
    assert counts["source"] == index["source"] == "5819ec8dc2d2ed897cc4716aa242131acef9f5c5"
    for kind, passed, ignored in (("binary", 21824, 69), ("doc", 27, 8)):
        observed = counts["summary"][kind]
        assert (observed["passed"], observed["failed"], observed["ignored"]) == (passed, 0, ignored)
    assert "check-all: 86 gate(s) clean" in records.read(prefix + "policy.txt").decode()
print("PASS: 284 archive members; 4746 inputs; 285 test artifacts; scoped counts and guards")
