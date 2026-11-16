#!/usr/bin/env python3
"""Read compact records and compare their identities; never run archived code."""
from pathlib import Path
import hashlib
import json
import re
import zipfile

root = Path(__file__).resolve().parent
index = json.loads((root / "wick-records.json").read_text())
archive = root / "wick-records.zip"
assert archive.stat().st_size == index["archive_bytes"] == 8070368
assert hashlib.sha256(archive.read_bytes()).hexdigest() == index["archive_sha256"]
with zipfile.ZipFile(archive) as z:
    members = index["members"]
    assert len(members) == len({r["name"] for r in members}) == 328
    assert set(z.namelist()) == {r["name"] for r in members}
    for row in members:
        raw = z.read(row["name"])
        assert len(raw) == row["bytes"]
        assert hashlib.sha256(raw).hexdigest() == row["sha256"]
        assert not raw.startswith(b"\x7fELF")
    def raw(name):
        return z.read("records/" + name)
    def data(name):
        return json.loads(raw(name))
    def checks(name, count):
        lines = raw(name).decode().splitlines()
        assert len(lines) == count and all(line.endswith(": OK") for line in lines)
    for prefix in ("privacy-mvm-full2", "privacy-controls1", "privacy-sdk3",
                   "privacy-sdk-custody2", "privacy-paired1"):
        assert all(v == 0 for v in data(prefix + "-outcome.json").values())
    for prefix, count, before, after in (
        ("privacy-mvm-full2", 4747, "file-set-before", "file-set-after"),
        ("privacy-controls1", 4747, "file-set-before", "file-set-after"),
        ("privacy-paired1", 17, "file-set-before", "file-set-after"),
    ):
        frozen = raw(prefix + "-frozen-inputs.sha256")
        assert len(frozen.splitlines()) == count
        assert frozen == raw(prefix + "-" + before + ".sha256") == raw(prefix + "-" + after + ".sha256")
        checks(prefix + "-inputs-before.txt", count)
        checks(prefix + "-inputs-after.txt", count)
    for prefix in ("privacy-sdk3", "privacy-sdk-custody2"):
        frozen = raw(prefix + "-frozen-inputs.sha256")
        assert len(frozen.splitlines()) == 490
        assert frozen == raw(prefix + "-sdk-file-set-before.sha256") == raw(prefix + "-sdk-file-set-after.sha256")
        for phase in ("before", "after"):
            checks(prefix + "-sdk-inputs-" + phase + ".txt", 490)
            checks(prefix + "-mvm-inputs-" + phase + ".txt", 4747)
    full = data("privacy-mvm-full2-test-counts.json")
    assert (full["summary"]["binary"]["passed"], full["summary"]["binary"]["failed"], full["summary"]["binary"]["ignored"]) == (21847, 0, 71)
    assert (full["summary"]["doc"]["passed"], full["summary"]["doc"]["failed"], full["summary"]["doc"]["ignored"]) == (27, 0, 8)
    artifacts = data("privacy-mvm-full2-executables-before.json")
    assert len(artifacts) == 285 and all(not r["fresh"] for r in artifacts)
    assert data("privacy-mvm-full2-runtime-executable-set.json") == {"expected": 285, "running_lines": 285, "unique_executed": 285, "exact_match": True}
    checks("privacy-mvm-full2-executables-after.txt", 285)
    assert "check-all: 86 gate(s) clean" in raw("privacy-mvm-full2-policy.txt").decode()
    for label in ("live", "adapter", "witness", "journal"):
        profiles = data("privacy-sdk3-" + label + "-profiles.json")
        build = [json.loads(line) for line in raw("privacy-sdk3-" + label + "-build.jsonl").decode().splitlines()]
        assert {r["name"] for r in profiles} == {"mvm-core", "mvm-crypto"}
        for row in profiles:
            selected = [r for r in build if r.get("reason") == "compiler-artifact" and r.get("package_id") == row["package_id"] and r["target"]["kind"] == ["lib"] and not r["profile"]["test"]]
            assert len(selected) == 1 and selected[0]["profile"] == row["profile"]
            assert row["fresh"] is False and selected[0]["fresh"] is False
            assert row["profile"]["opt_level"] == "2" and row["profile"]["overflow_checks"] and row["profile"]["debug_assertions"]
        suffix = "profile-artifacts-after.txt"
        checks("privacy-sdk3-" + label + "-" + suffix, 4)
    result = re.compile(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
    assert result.findall(raw("privacy-sdk3-live-tests.txt").decode())[-1] == ("175", "0", "1")
    for label, count, targets in (("owner", 29, 1), ("supervisor", 74, 6)):
        prefix = "privacy-sdk-custody2-" + label
        runtime = data(prefix + "-runtime.json")
        files = data(prefix + "-executables-before.json")
        assert runtime["passed"] == count and runtime["ignored"] == 0 and runtime["exact_compiler_set"]
        assert len(files) == len(runtime["actual_paths"]) == targets
        assert {r["path"] for r in files} == set(runtime["actual_paths"])
        assert all(r["fresh"] is True for r in files)
        checks(prefix + "-executables-after.txt", targets)
    paired = data("privacy-paired1-executables-before.json")
    assert len(paired) == 4 and all(not r["fresh"] for r in paired)
    assert data("privacy-paired1-runtime-executable-set.json") == {"compiled_targets": 4, "actual_running_targets": 4, "exact_match": True}
    checks("privacy-paired1-executables-after.txt", 4)
    cases = result.findall(raw("privacy-paired1-tests.txt").decode())
    assert sum(int(r[0]) for r in cases) == 18 and sum(int(r[1]) for r in cases) == 0 and sum(int(r[2]) for r in cases) == 1
    assert data("privacy-sdk1-outcome.json")["original_exit"] == 101
    assert data("privacy-sdk2-outcome.json")["original_exit"] == 1
    assert raw("privacy-sdk2-format.txt")
    cleanup = [data("privacy-stale-compiler-cleanup.json"), data("privacy-stale-compiler2-cleanup.json")]
    assert sum(r["deleted_files"] for r in cleanup) == 3292
    assert sum(r["deleted_apparent_bytes"] for r in cleanup) == 12530456805
    for item in cleanup:
        assert item["lsof_exit"] == 1 and not item["lsof_stdout"]
        assert all(Path(r["path"]).suffix in {".rlib", ".rmeta"} for r in item["files"])

receipts = {label: json.loads((root / (label + "-commit.json")).read_text()) for label in ("reviewed-head", "ci-merge", "release", "mvm-source")}
assert {receipts[label]["commit"]["tree"]["sha"] for label in ("reviewed-head", "ci-merge", "release")} == {"9edebbbb8d22e473ce75e28eb6bd8104c9f0b2a9"}
assert receipts["mvm-source"]["sha"] == "e95f1119dac66e45a81c4eead8f4e747a1286a86"
assert receipts["mvm-source"]["commit"]["tree"]["sha"] == "1642e7294aee39c51a634c9d10d0b25d7bccdb5e"
print("PASS: 328 records; frozen guards, target/profile/custody counts, failures, cleanup and release-tree receipts")
