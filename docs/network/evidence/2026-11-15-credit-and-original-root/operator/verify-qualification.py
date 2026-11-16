"""Verify retained qualification records, not live runtime acceptance."""
import hashlib
import json
import pathlib
import re
import shlex
import subprocess
import sys
import zipfile

operator = pathlib.Path(__file__).resolve().parent
checkout = pathlib.Path(sys.argv[1]).resolve()


def manifest(raw):
    rows = {}
    for line in raw.decode().splitlines():
        digest, name = line.split(None, 1)
        assert re.fullmatch(r'[a-f0-9]{64}', digest), line
        assert name not in rows, name
        rows[name] = digest
    return rows


with zipfile.ZipFile(operator / 'wick-records.zip') as z:
    text = lambda name: z.read(name).decode()
    data = lambda name: json.loads(z.read(name))
    mvm = manifest((operator / 'frozen-mvm-inputs.sha256').read_bytes())
    root = manifest((operator / 'frozen-root-inputs.sha256').read_bytes())
    assert len(mvm) == 4717 and len(root) == 490
    assert manifest(z.read('admitted2-final-inputs.sha256')) == mvm
    assert manifest(z.read('sc-root-b2-fixed-inputs.sha256')) == root
    for name, digest in root.items():
        raw = subprocess.check_output([
            'git', '-C', str(checkout), 'show',
            'ee02e14e9b44bf530f485036e9a3708c87f556d4:' + name.removeprefix('./')])
        assert hashlib.sha256(raw).hexdigest() == digest, name

    def guarded(prefix, inputs):
        for phase in ('before', 'after'):
            lines = text(prefix + '-inputs-' + phase + '.txt').splitlines()
            names = []
            for line in lines:
                assert line.endswith(': OK'), line
                name = line[:-4]
                if name not in inputs:
                    # The original guard shell-quotes three glob-bearing paths.
                    parsed = shlex.split(name)
                    assert len(parsed) == 1 and shlex.quote(parsed[0]) == name
                    name = parsed[0]
                names.append(name)
            assert len(names) == len(inputs) and set(names) == set(inputs)

    guarded('admitted2-mvm-full', mvm)
    guarded('root-b2-qualified2', root)
    guarded('root-b2-qualified2-mvm', mvm)
    for prefix in ('admitted2-mvm-full', 'root-b2-qualified2', 'root-b2-qualified2-mvm'):
        assert z.read(prefix + '-file-set-before.sha256') == z.read(prefix + '-file-set-after.sha256')
    for prefix in ('admitted2-mvm-full', 'admitted-sdk-final', 'root-b2-qualified2'):
        assert text(prefix + '-exit.txt').strip() == '0'
    assert 'check-all: 86 gate(s) clean' in text('admitted2-mvm-full-policy.txt')

    exes = data('admitted2-mvm-full-executables-before.json')
    before = manifest(z.read('admitted2-mvm-full-executables-before.sha256'))
    assert len(exes) == len(before) == 284
    assert {x['path']: x['sha256'] for x in exes} == before
    assert sum(not x['fresh'] for x in exes) == 260
    assert all(not x['prior_root_occurrences'] for x in exes)
    assert all(not x['fresh'] or x['same_root_reuse_verified'] for x in exes)
    assert sum(x['bytes'] for x in exes) == 3781944272
    assert set(text('admitted2-mvm-full-executables-after.txt').splitlines()) == {
        name + ': OK' for name in before}
    running = re.findall(r'^     Running .* \((.+)\)$', text('admitted2-mvm-full-tests.txt'), re.M)
    assert len(running) == 284 and set(running) == set(before)

    # Recompute outer harness counts; nested binary test-output examples are
    # retained in their groups but only the outer final footer counts.
    groups = []
    current = None
    footer = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
    for line in text('admitted2-mvm-full-tests.txt').splitlines():
        if line.startswith('     Running ') or line.startswith('   Doc-tests '):
            if current:
                groups.append(current)
            current = {'kind': 'binary' if line.startswith('     Running ') else 'doc',
                       'target': line.strip(), 'summaries': []}
        elif current:
            match = footer.match(line)
            if match:
                current['summaries'].append(dict(status=match[1], passed=int(match[2]),
                                                 failed=int(match[3]), ignored=int(match[4])))
    assert current
    groups.append(current)
    assert groups == data('admitted2-mvm-full-test-counts.json')['groups']
    assert all(g['summaries'] for g in groups)
    counts = {}
    for kind in ('binary', 'doc'):
        selected = [g for g in groups if g['kind'] == kind]
        rows = [row for g in selected for row in (g['summaries'][-1:] if kind == 'binary' else g['summaries'])]
        assert all(row['status'] == 'ok' and row['failed'] == 0 for row in rows)
        counts[kind] = {key: sum(row[key] for row in rows) for key in ('passed', 'failed', 'ignored')}
    assert counts == {'binary': {'passed': 21814, 'failed': 0, 'ignored': 69},
                      'doc': {'passed': 27, 'failed': 0, 'ignored': 8}}

    labels = ('live', 'adapter', 'journal', 'owner', 'supervisor', 'witness', 'guest', 'mvm')
    for label in labels:
        assert data('credit-version-graphs/' + label + '-after.json') == data(
            'admitted-final-graphs/' + label + '-after.json')
        assert data('admitted-final-graphs/' + label + '-summary.json')['source'] == (
            'b2def5fd556e6ab9aabeb281b554c6ef6bb060d9')

    for prefix in ('root-b2-qualified1', 'root-b2-qualified2'):
        value = data(prefix + '-measurements.json')
        assert value['exact_blocks'] == value['populated_store']['distinct_publications'] == 64
        assert value['app_bytes_each_direction'] == value['peer']['app_bytes_each_direction'] == 1048576
        assert (value['guest_load_sealed_request_bytes'], value['guest_load_sealed_response_bytes']) == (1078631, 1078502)
        assert (value['peer']['tls_received'], value['peer']['tls_sent']) == (103046, 102623)
        assert value['host_ceiling_bytes'] == 2147483648 and value['worker_ceiling'] == 24
        samples = value['process_samples']
        assert len(samples) == 64
        assert len({(x['pid'], x['start_ticks']) for x in samples}) == 1
        assert {x['address_space_limit'] for x in samples} == {295698432}
        assert max(x['threads'] for x in samples) == 7
        assert max(x['original_store_open_fds'] for x in samples) == 3
        exes = manifest(z.read(prefix + '-executables-before.sha256'))
        assert len(exes) == 4
        assert set(text(prefix + '-executables-after.txt').splitlines()) == {n + ': OK' for n in exes}
    assert text('credit-red-exit.txt').strip() == '101'
    assert 'round 16, unexpected GoAway: stream 1 sent 16384 bytes with only 0 credit' in text('credit-red-tests.txt')
    assert text('admitted-mvm-full-exit.txt').strip() == '101'
    assert text('root-b2-clippy1-exit.txt').strip() == '101'

print(json.dumps(dict(scope='retained records only; no live VM/WAN acceptance',
                      root_committed_inputs=490, mvm_guarded_inputs=4717,
                      actual_running_executables=284, exact_graph_pairs=8,
                      corrected_source_test_counts=counts)))
