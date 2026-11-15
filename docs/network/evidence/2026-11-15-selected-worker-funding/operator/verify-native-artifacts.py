import hashlib
import json
import pathlib
import re
import sys
import zipfile

root = pathlib.Path(sys.argv[1])
sc = pathlib.Path(sys.argv[2])
mvm_manifest = pathlib.Path(sys.argv[3])
catalog = json.loads((root / 'catalog.json').read_text())
head = '3f7ca46fc26b411bd07ac9e9bf7d3fa867f4f42c'
merge = '602402a0e22c83971e175461ebfe73af82369340'
mvm = '8345cb682ecbc755c7312ba7a5a28c641aa667d0'
tree = '9b6fe6888a499b3f7dc4f89d92dcdc558fed62fe'
core = '8994951b7c13fe5acf6f5ea3060ad4e51dc473c2'
assert catalog['head'] == head and catalog['ci_merge'] == merge
sha_line = re.compile(r'^([a-fA-F0-9]{64})\s+\*?(.+?)\s*$')
result_line = re.compile(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', re.M)

def checksums(text):
    rows = {}
    for line in text.splitlines():
        match = sha_line.fullmatch(line)
        assert match, line
        name = match[2].replace('\\', '/').removeprefix('./')
        assert name not in rows, name
        rows[name] = match[1].lower()
    return rows

expected_mvm = checksums(mvm_manifest.read_text())
assert len(expected_mvm) == 4716
audits = []
measurements = []
for artifact in catalog['artifacts']:
    path = root / artifact['file_name']
    data = path.read_bytes()
    assert len(data) == artifact['expected_size']
    assert hashlib.sha256(data).hexdigest() == artifact['expected_sha256']
    audit = {'id': artifact['id'], 'name': artifact['name'], 'zip_bytes': len(data),
             'embedded_member_hashes_verified': 0, 'source_hashes_verified': 0}
    with zipfile.ZipFile(path) as z:
        names = z.namelist()
        assert len(names) == len(set(names))
        assert z.testzip() is None
        for name in names:
            normalized = name.replace('\\', '/')
            assert not normalized.startswith('/') and '..' not in pathlib.PurePosixPath(normalized).parts
        audit['members'] = len(names)
        text = lambda name: z.read(name).decode('utf-8-sig')
        for name in names:
            if pathlib.PurePosixPath(name).name == 'SHA256SUMS':
                for target, digest in checksums(text(name)).items():
                    matches = [n for n in names if n == target or n.endswith('/' + target)]
                    assert len(matches) == 1, (artifact['name'], target)
                    assert hashlib.sha256(z.read(matches[0])).hexdigest() == digest
                    audit['embedded_member_hashes_verified'] += 1
        if 'source-commit.txt' in names:
            assert text('source-commit.txt').strip() in (head, merge)
        if 'identity.json' in names:
            identity = json.loads(text('identity.json'))
            if artifact['name'].startswith('flowmux-component-'):
                assert identity['source'] in (head, merge), identity
                assert str(identity['run']) == str(artifact['run_id'])
            elif artifact['name'].startswith('mvm-mailbox-portable-'):
                assert identity['source_commit'] in (head, merge), identity
                assert identity['github_sha'] in (head, merge), identity
                assert str(identity['run_id']) == str(artifact['run_id'])
                resolved = json.loads(text('mvm-source.json'))
                assert len(resolved) == 2
                assert {p['name'] for p in resolved} == {'mvm-core', 'mvm-contract'}
                assert all(p['source'] == f'git+https://github.com/Casey-allard/mvm?rev={mvm}#{mvm}' for p in resolved)
            else:
                raise AssertionError(('unrecognized identity schema', artifact['name']))
        if artifact['name'].startswith('native-store-consumer-'):
            assert text('source-and-tree.txt').splitlines() == [mvm, tree]
            assert text('declared-source.txt').strip() == f'git+https://github.com/Casey-allard/mvm?rev={mvm}#{mvm}'
            assert checksums(text('source.sha256')) == expected_mvm
            audit['source_hashes_verified'] = len(expected_mvm)
            expected_after = {name + ': OK' for name in expected_mvm}
            assert set(text('source-after.txt').splitlines()) == expected_after
            assert text('executable-before.sha256') == text('executable-after.sha256')
            assert text('shared-core-lock-sources.txt').splitlines() == [f'source = "git+https://github.com/R4-Laboratories/SpiralCore?rev={core}#{core}"'] * 5
            assert text('expected-lock-names.txt') == text('shared-core-lock-names.txt')
            for i in range(8):
                counts = result_line.findall(text(f'control-{i}.txt'))
                assert counts == [('1', '0', '0')], (artifact['name'], i, counts)
            audit['original_controls_passed'] = 8
            assert result_line.findall(text('control-8.txt')) == [('1', '0', '0')]
            assert 'executed_controls=9' in text('counts.txt').splitlines()
            assert 'installed_endpoint::controller::network::worker_tests::serial_worker_quote_includes_all_selected_concurrent_owners: test' in text('test-list.txt').splitlines()
            assert text('stack-configured-kib.txt').strip() == '8192'
            audit['worker_quote_controls_passed'] = 1
            audit['executed_controls_passed'] = 9
            audit['executable_identity'] = 'runner-recorded equal before/after; executable bytes are not in ZIP'
        elif 'source.sha256' in names:
            source_hashes = checksums(text('source.sha256'))
            for name, digest in source_hashes.items():
                assert hashlib.sha256((sc / name).read_bytes()).hexdigest() == digest, name
            audit['source_hashes_verified'] = len(source_hashes)
            if 'source-after.txt' in names:
                assert set(text('source-after.txt').splitlines()) == {n + ': OK' for n in source_hashes}
        if artifact['name'].startswith('canonical-endpoint-') and 'linux' in artifact['name']:
            assert result_line.findall(text('worker-tests.txt')) == [('2', '0', '0')]
            assert set(text('source-after-workers.txt').splitlines()) == {n + ': OK' for n in checksums(text('source.sha256'))}
            audit['original_worker_ledger_controls_passed'] = 2

        if artifact['name'].startswith('flowmux-component-'):
            counts = json.loads(text('counts.json'))
            ordered = json.loads(text('ordered-counts.json'))
            assert (counts['passed'], counts['failed'], counts['ignored'], counts['listed']) == (89, 0, 0, 89)
            assert (ordered['passed'], ordered['failed'], ordered['ignored'], ordered['listed']) == (20, 0, 0, 20)
            bulk = [line[line.index('ORDERED_BULK '):] for line in text('tests.txt').splitlines() if 'ORDERED_BULK ' in line]
            assert len(bulk) == 1
            values = {}
            for match in re.finditer(r'(\w+)=(Some\([-+]?\d+\)|None|[-+]?\d+)', bulk[0]):
                value = match[2]
                values[match[1]] = None if value == 'None' else int(value[5:-1] if value.startswith('Some(') else value)
            assert values['exact_bytes_each_direction'] == 67108864
            assert values['actual_tls_a_sent'] == values['peer_received']
            assert values['actual_tls_a_received'] == values['peer_sent']
            assert values['actual_tls_a_sent'] + values['actual_tls_a_received'] == 10790065
            assert values['lifecycle_wall_us'] < 240_000_000
            observations = {'artifact': artifact['name'], 'scope': 'native paired TCP/TLS host processes; no MVM backend or WAN',
                            'observations': values, 'raw': bulk[0], 'identity': json.loads(text('identity.json')),
                            'counts': counts, 'ordered_counts': ordered}
            measurements.append(observations)
    audits.append(audit)
summary = {'archives': len(audits), 'zip_bytes': sum(x['zip_bytes'] for x in audits),
           'members': sum(x['members'] for x in audits),
           'embedded_member_hashes_verified': sum(x['embedded_member_hashes_verified'] for x in audits),
           'source_hashes_verified_against_frozen_inputs': sum(x['source_hashes_verified'] for x in audits),
           'native_flowmux_rows': len(measurements), 'audits': audits}
assert len(measurements) == 7
if len(sys.argv) > 4 and sys.argv[4] == '--write':
    (root / 'validation.json').write_text(json.dumps(summary, indent=2) + '\n')
    (root.parent / 'native-measurements.json').write_text(json.dumps(measurements, indent=2) + '\n')
else:
    assert json.loads((root / 'validation.json').read_text()) == summary
    assert json.loads((root.parent / 'native-measurements.json').read_text()) == measurements
print(json.dumps({k: v for k, v in summary.items() if k != 'audits'}))
for row in measurements:
    v = row['observations']
    print(json.dumps({'artifact': row['artifact'], 'lifecycle_wall_us': v['lifecycle_wall_us'],
                      'lifecycle_process_cpu_us': v.get('lifecycle_process_cpu_us'), 'setup_wall_us': v['setup_wall_us']}))
