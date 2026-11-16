import hashlib
import json
import pathlib
import re
import sys
import subprocess
import zipfile

root = pathlib.Path(sys.argv[1])
sc = pathlib.Path(sys.argv[2])
mvm_manifest = pathlib.Path(sys.argv[3])
catalog = json.loads((root / 'catalog.json').read_text())
head = 'ee02e14e9b44bf530f485036e9a3708c87f556d4'
merge = '9c221043e95dbba142fb16caf785ec64956ed0bc'
mvm = 'b2def5fd556e6ab9aabeb281b554c6ef6bb060d9'
tree = '81a712066d4c7dfb28e48cfd718c93e39ee4c47c'
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
assert len(expected_mvm) == 4717
audits = []
measurements = []
root_measurements = []
sc_cache = {}
def sc_digest(name):
    if name not in sc_cache:
        data = subprocess.check_output(["git", "-C", str(sc), "show", head + ":" + name])
        sc_cache[name] = hashlib.sha256(data).hexdigest()
    return sc_cache[name]
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
            assert 'executed_controls=10' in text('counts.txt').splitlines()
            assert result_line.findall(text('control-9.txt')) == [('1', '0', '0')]
            assert 'supervisor::endpoint::flowmux::credit_tests::returned_upload_credit_keeps_the_session_validator_current: test' in text('test-list.txt').splitlines()
            assert re.search(r'^STORE_DROP_ORDER called=true ledger_held=true returned_credit_visible=false original_owner_released=true retirement_wall_ns=[0-9]+$', text('control-6.txt'), re.M)
            audit['returned_credit_controls_passed'] = 1
            assert 'installed_endpoint::controller::network::worker_tests::serial_worker_quote_includes_all_selected_concurrent_owners: test' in text('test-list.txt').splitlines()
            assert text('stack-configured-kib.txt').strip() == '8192'
            audit['worker_quote_controls_passed'] = 1
            audit['executed_controls_passed'] = 10
            audit['executable_identity'] = 'runner-recorded equal before/after; executable bytes are not in ZIP'
        elif 'source.sha256' in names:
            source_hashes = checksums(text('source.sha256'))
            for name, digest in source_hashes.items():
                assert sc_digest(name) == digest, name
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
        if artifact['name'].startswith('canonical-endpoint-'):
            initial = [json.loads(line) for line in text('build.jsonl').splitlines()]
            fixtures = [r for r in initial if r.get('reason') == 'compiler-artifact' and r.get('profile', {}).get('test') and r.get('target', {}).get('name') == 'spiralcore_mvm_live' and r.get('executable')]
            assert len(fixtures) == 1 and fixtures[0]['fresh'] is False
            executable = fixtures[0]['executable']
            recorded = checksums(text('executable-before.sha256'))
            assert set(recorded) == {executable}
            assert text('executable-after.txt').splitlines() == [executable + ': OK']
            assert result_line.findall(text('tests.txt')) == [('8', '0', '0')]
            if 'linux' in artifact['name']:
                assert text('root-exit.txt').strip() == '0'
                assert 'no VM boot or WAN' in text('root-scope.txt')
                assert text('root-mvm-source-and-tree.txt').splitlines() == [mvm, tree]
                assert text('root-mvm-clean-before.txt') == text('root-mvm-clean-after.txt') == ''
                for name in ['root-mvm-source.sha256', 'root-mvm-file-set-before.sha256', 'root-mvm-file-set-after.sha256']:
                    assert checksums(text(name)) == expected_mvm
                assert set(text('root-mvm-source-after.txt').splitlines()) == {n + ': OK' for n in expected_mvm}
                audit['root_dependency_hashes_verified'] = len(expected_mvm)
                audit['source_hashes_verified'] += len(expected_mvm)
                assert set(text('source-after-root-store.txt').splitlines()) == {n + ': OK' for n in checksums(text('source.sha256'))}
                four = checksums(text('root-executables-before.sha256'))
                assert len(four) == 4 and four[executable] == recorded[executable]
                assert set(text('root-executables-after.txt').splitlines()) == {n + ': OK' for n in four}
                assert text('root-fixture-before.txt').splitlines() == [executable + ': OK']
                metadata = json.loads(text('root-metadata.json'))
                hostd = [p for p in metadata['packages'] if p['name'] == 'mvm-hostd']
                assert len(hostd) == 1 and hostd[0]['source'] == f'git+https://github.com/Casey-allard/mvm?rev={mvm}#{mvm}'
                dependency_root = hostd[0]['manifest_path'].removesuffix('/crates/mvm-hostd/Cargo.toml')
                helpers = {}
                for line in text('root-helper-build.jsonl').splitlines():
                    item = json.loads(line)
                    if item.get('reason') == 'compiler-artifact' and item.get('executable') and item['target']['name'] in ('mvm-host-agent', 'mvm-signer-helper', 'mvm-network-endpoint'):
                        assert item['target']['src_path'].startswith(dependency_root + '/')
                        helpers[item['target']['name']] = item['executable']
                assert len(helpers) == 3 and set(helpers.values()) == set(four) - {executable}
                raw = text('root-store-tests.txt')
                assert result_line.findall(raw) == [('1', '0', '0')]
                values = [json.loads(line) for line in raw.splitlines() if line.startswith('{')]
                assert len(values) == 1
                value = values[0]
                assert value['exact_blocks'] == 64 and value['app_bytes_each_direction'] == 1048576
                assert value['populated_store']['distinct_publications'] == 64
                assert len(value['populated_store']['records']) == 64
                samples = value['process_samples']
                assert len(samples) == 64 and len({(x['pid'], x['start_ticks']) for x in samples}) == 1
                assert all(x['address_space_limit'] == 295698432 and x['virtual_bytes'] <= x['address_space_limit'] and x['threads'] <= 7 and x['original_store_open_fds'] <= 7 for x in samples)
                assert max(x['threads'] for x in samples) == 7
                assert value['host_ceiling_bytes'] == 2147483648 and value['worker_ceiling'] == 24
                assert value['after_endpoint']['usage']['guests'] == 1 and value['after_endpoint']['usage']['workers'] >= 1
                assert value['vm_retirement'].startswith('NOT_RUN')
                assert value['peer']['app_bytes_each_direction'] == 1048576
                before, charged, after = (value[k]['usage'] for k in ('before', 'charged', 'after_original_endpoint_cleanup'))
                assert charged['workers'] - before['workers'] == 7
                assert charged['host_metadata_bytes'] - after['host_metadata_bytes'] == 4096
                assert charged['codec_store_file_descriptors'] == after['codec_store_file_descriptors']
                assert charged['workers'] == after['workers']
                root_measurements.append({'artifact': artifact['name'], 'run_id': artifact['run_id'], 'scope': 'native original Root and selected Linux host process; no VM boot or WAN', 'measurement': value, 'whole_fixture_gnu_time': text('root-store-time.txt'), 'compiler_fixture_fresh': fixtures[0]['fresh'], 'executable_identity': 'runner-recorded before/after; executable bytes absent from ZIP'})
                audit['original_root_exact_tests_passed'] = 1
    audits.append(audit)
summary = {'archives': len(audits), 'zip_bytes': sum(x['zip_bytes'] for x in audits),
           'members': sum(x['members'] for x in audits),
           'embedded_member_hashes_verified': sum(x['embedded_member_hashes_verified'] for x in audits),
           'source_hashes_verified_against_frozen_inputs': sum(x['source_hashes_verified'] for x in audits),
           'native_flowmux_rows': len(measurements), 'native_original_root_rows': len(root_measurements), 'audits': audits}
assert len(measurements) == 0
assert len(root_measurements) == 2
if len(sys.argv) > 4 and sys.argv[4] == '--write':
    (root / 'validation.json').write_text(json.dumps(summary, indent=2) + '\n')
    (root / 'native-measurements.json').write_text(json.dumps(root_measurements, indent=2) + '\n')
else:
    assert json.loads((root / 'validation.json').read_text()) == summary
    assert json.loads((root / 'native-measurements.json').read_text()) == root_measurements
print(json.dumps({k: v for k, v in summary.items() if k != 'audits'}))
for row in measurements:
    v = row['observations']
    print(json.dumps({'artifact': row['artifact'], 'lifecycle_wall_us': v['lifecycle_wall_us'],
                      'lifecycle_process_cpu_us': v.get('lifecycle_process_cpu_us'), 'setup_wall_us': v['setup_wall_us']}))

for row in root_measurements:
    v = row['measurement']
    print(json.dumps({'artifact': row['artifact'], 'exact_blocks': v['exact_blocks'], 'setup_wall_ns': v['setup_wall_ns'], 'verified_wall_ns': v['verified_wall_ns'], 'sender_thread_cpu_ns': v['sender_thread_cpu_ns'], 'child_cpu_ns': v['endpoint_whole_child_cpu_collected_at_retirement_ns'], 'peer_thread_cpu_ns': v['peer']['thread_cpu_ns'], 'peer_tls': [v['peer']['tls_received'], v['peer']['tls_sent']], 'sealed_load': [v['guest_load_sealed_request_bytes'], v['guest_load_sealed_response_bytes']]}))
