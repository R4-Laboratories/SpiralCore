# Independent SC-OPT-003 qualification review method

This is a read-only artifact check performed before the paired measurement. It does
not compile, execute the probe, rerun tests, or produce a timing sample. The frozen
production source is `80449939bd92348593cc88555cf7fde0847ceffd`. The reviewed
measurement Rust is byte-identical to SC-OPT-002; the helper's approved default-off
feature alias is its only manifest change beyond the expanded dependency lock.

The checks below use Python's `hashlib` as an independent analysis implementation
of the published SHA-256 domain framing. Python is used only to inspect receipts
and bytes, not as production protocol code or as the benchmark implementation.
The cross-backend check compares complete files, not just their digest strings.

Run from `/Users/casey.allard/spiralcore/local/sha-efficiency-20261029`:

```sh
python3 - <<'PY'
import hashlib, json, pathlib, re
p = pathlib.Path('.').resolve()
repo = pathlib.Path('/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore')
candidate = '80449939bd92348593cc88555cf7fde0847ceffd'
sha = lambda b: hashlib.sha256(b).digest()
file_sha = lambda path: sha(path.read_bytes()).hex()

def load(path):
    return json.loads(path.read_text())

def check_manifest(path, root):
    lines = path.read_text().splitlines()
    for line in lines:
        expected, name = line.split('  ', 1)
        target = pathlib.Path(name)
        if not target.is_absolute():
            target = root / target
        assert file_sha(target) == expected, str(target)
    return len(lines)

q = load(p / 'qualification.json')
assert q['status'] == 'passed' and q['source_revision'] == candidate
assert q['identity_outputs_equal'] and q['probe_tests_both']
assert q['identity_qualification_sha256'] == file_sha(p / 'identity-qualification.json')
assert q['identity_execution_sha256'] == file_sha(p / 'identity-execution.jsonl')

for variant, features in (
    ('software', ['default', 'std']),
    ('hardware', ['asm', 'default', 'sha2-asm', 'std']),
):
    d = p / ('build-' + variant)
    build = load(d / 'build.json')
    assert build['source_revision'] == candidate and build['variant'] == variant
    assert build['native_target'] == 'aarch64-apple-darwin'
    assert build['production_source_fingerprint_sha256'] == file_sha(d / 'production-source-files.sha256')
    assert check_manifest(d / 'executables.sha256', p) == 2
    assert check_manifest(d / 'helper-files.sha256', p) == 13
    check_manifest(d / 'production-source-files.sha256', repo)
    metadata = load(d / 'metadata.json')
    selected = [sorted(n['features']) for n in metadata['resolve']['nodes']
                if '#sha2@0.10.9' in n['id']]
    assert selected == [features]
    c = q['configurations'][variant]
    assert c['receiver_probe_sha256'] == file_sha(d / 'receiver-probe')
    assert c['identity_executable_sha256'] == file_sha(d / 'identity')
    assert c['helper_manifest_sha256'] == file_sha(d / 'helper-files.sha256')
    assert c['tests_passed'] == 7 and c['tests_failed'] == 0
    # This portion was checked before the requested diagnostic-log cleanup.
    # If the log has subsequently been removed, its original hash and result are
    # retained in the qualification/review receipts; do not claim a fresh log check.
    log = p / 'logs' / ('probe-' + variant + '-tests.log')
    if log.exists():
        assert c['test_log_sha256'] == file_sha(log)
        assert re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed',
                          log.read_text()) == [('ok', '7', '0')]
    else:
        print('historical diagnostic log unavailable:', log)
    assert check_manifest(p / ('identity-' + variant + '.sha256'),
                          p / ('identity-' + variant)) == 37

for name in ('helper-files.sha256', 'production-source-files.sha256',
             'compiler.txt', 'c-compiler.txt', 'build-tools.sha256',
             'tool-selection.txt', 'apple-tools-package.txt', 'build-environment.txt'):
    assert (p / 'build-software' / name).read_bytes() == (p / 'build-hardware' / name).read_bytes(), name

software_dir, hardware_dir = p / 'identity-software', p / 'identity-hardware'
assert {f.name for f in software_dir.iterdir()} == {f.name for f in hardware_dir.iterdir()}
assert len(list(software_dir.iterdir())) == 37
for f in software_dir.iterdir():
    assert f.read_bytes() == (hardware_dir / f.name).read_bytes(), f.name

rows = [json.loads(line) for line in (software_dir / 'identities.jsonl').read_text().splitlines()]
assert len(rows) == 36 and len({r['case'] for r in rows}) == 36
assert all(r['exact_rebuild'] for r in rows)
checked_nodes = 0
for row in rows:
    data = (software_dir / row['nodes_file']).read_bytes()
    assert sha(data).hex() == row['node_serialization_sha256']
    offset, previous, nodes = 0, None, {}
    while offset < len(data):
        assert len(data) - offset >= 4
        length = int.from_bytes(data[offset:offset + 4], 'little')
        offset += 4
        encoded = data[offset:offset + length]
        offset += length
        assert length in (66, 130) and len(encoded) == length
        node_id, body = encoded[:32], encoded[32:]
        assert node_id == sha(b'spiralcore.object.node.v1\0' + body)
        assert previous is None or previous < node_id
        previous = node_id
        assert node_id not in nodes
        nodes[node_id] = body
        checked_nodes += 1
    assert offset == len(data) and len(nodes) == row['node_count']
    manifest = bytes(row['manifest_bytes'])
    assert sha(b'spiralcore.object.manifest.v1\0' + manifest).hex() == row['manifest_id']
    assert int.from_bytes(manifest[:2], 'little') == 1
    assert len(manifest) == 43 + 33 * manifest[42]
    length = int.from_bytes(manifest[2:10], 'little')
    assert length == row['bytes']

    def rebuild(node_id, scale):
        body = nodes[node_id]
        assert body[1] == scale
        if scale == 0:
            assert len(body) == 34 and body[0] == 0
            return body[2:]
        assert len(body) == 98 and body[0] == 1
        return b''.join(rebuild(body[2 + i * 32:34 + i * 32], scale - 1)
                        for i in range(3))

    result = b''.join(rebuild(manifest[i + 1:i + 33], manifest[i])
                      for i in range(43, len(manifest), 33))[:length]
    assert len(result) == length and sha(result).hex() == row['input_sha256']
    object_id = sha(b'spiralcore.object.bytes.v1\0' + length.to_bytes(8, 'little') + result)
    assert object_id == manifest[10:42] and object_id.hex() == row['object_id']
assert checked_nodes == 113748
known = {r['case']: r['object_id'] for r in rows}
assert known['receiver-base'] == 'c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0'
assert known['receiver-target'] == '227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf'

events = [json.loads(line) for line in (p / 'identity-execution.jsonl').read_text().splitlines()]
assert len(events) == 6
for variant in ('software', 'hardware'):
    selected = [e for e in events if e['configuration'] == variant]
    assert [e['event'] for e in selected] == ['start', 'end', 'negative_checks']
    assert selected[1]['exit_code'] == 0
    negative = selected[2]
    assert negative['reuse_exit'] == 1 and negative['relative_exit'] == 1
    assert negative['original_files_unchanged'] and negative['relative_path_not_created']
    assert not (p / ('identity-relative-' + variant)).exists()
print(json.dumps({'fixtures': len(rows), 'node_ids_verified': checked_nodes,
                  'complete_identity_files_equal': 37,
                  'manifest_ids_and_full_rebuilds_verified': len(rows),
                  'binary_helper_qualification_bindings_verified': True}))
PY
```

The preserved negative events are recorded executions, not a second execution by
the reviewer. The file comparisons independently confirm that the resulting files
match the retained digests. Standard SHA-256 fixed-vector and protocol tests are
separate production qualification performed by the parent; the reviewer did not
rerun them. This finite fixture check is not a proof over every input, a FIPS
validation, unsupported-CPU/platform qualification, or a performance/energy result.
No network or paired campaign outcome is included in this premeasurement review.
