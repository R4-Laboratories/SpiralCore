"""Read retained records and hashes; never execute their code or extract files."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import tarfile

root = Path(__file__).resolve().parent
identity = json.loads((root / 'archive-identity.json').read_text())
archive = root / identity['file']
assert archive.stat().st_size == identity['bytes'] == 1528676
assert hashlib.sha256(archive.read_bytes()).hexdigest() == identity['sha256']
index = json.loads((root / 'records-index.json').read_text())['files']
expected = {r['path']: r for r in index}
assert len(index) == len(expected) == identity['members'] == 507
assert sum(r['bytes'] for r in index) == identity['uncompressed_file_bytes'] == 88972212
records = {}
process = subprocess.Popen(['zstd', '-qdc', str(archive)], stdout=subprocess.PIPE)
with tarfile.open(fileobj=process.stdout, mode='r|') as tar:
    for member in tar:
        assert member.isfile() and member.name in expected and member.name not in records
        assert not member.name.startswith('/') and '..' not in Path(member.name).parts
        row = expected[member.name]
        assert member.size == row['bytes'] < 30_000_000
        value = tar.extractfile(member).read()
        assert hashlib.sha256(value).hexdigest() == row['sha256']
        assert not value.startswith(b'\x7fELF')
        records[member.name] = value
assert process.wait() == 0 and set(records) == set(expected)


def text(name):
    return records[name].decode()


def data(name):
    return json.loads(records[name])


def hashes(name):
    rows = [line.split('  ', 1) for line in text(name).splitlines()]
    result = {path: digest for digest, path in rows}
    assert len(result) == len(rows)
    return result


quoted = {
    "'" + name + "'": name for name in (
        './public/src/pages/[...path].md.ts',
        './public/src/pages/blog/[...slug].astro',
        './public/src/pages/use-cases/[...slug].astro',
    )
}


def checks(name, paths):
    rows = text(name).splitlines()
    assert len(rows) == len(paths) and all(r.endswith(': OK') for r in rows), name
    assert {quoted.get(r[:-4], r[:-4]) for r in rows} == set(paths), name


def clean(name):
    assert all(v == 0 for v in data(name).values()), name


source = '1298ffe9b3cfa45cecb56cba8e328de00f845987'
sc = '448da7a47e2144211424d7b4c35a65669b953627'
mvm = hashes('sc-retention-green4-frozen-inputs.sha256')
sdk = hashes('sc-retention-sdk-frozen-inputs.sha256')
paired = hashes('sc-retention-paired-frozen-inputs.sha256')
fixtures = hashes('sc-retention-paired-fixtures.sha256')
assert (len(mvm), len(sdk), len(paired), len(fixtures)) == (4758, 490, 17, 5)
assert data('retention-full1-outcome.json') == dict(
    controller_exit=1, source_hash_exit=0, enumeration_exit=0, file_set_exit=0, final_exit=1)
assert 'check-all: 1 of 86 gate(s) failed:' in text('retention-full1-policy.txt')
assert 'spawning uvx datamodel-codegen' in text('retention-full1-policy.txt')
assert 'check-stubs: no drift' in text('retention-stubs1-policy.txt')
for label in ('retention-full2', 'retention-stubs1', 'retention-green4'):
    clean(label + '-outcome.json')
for label in ('retention-full1', 'retention-full2'):
    assert text(label + '-source.txt').strip() == source
    for side in ('before', 'after'):
        assert hashes(label + '-file-set-' + side + '.sha256') == mvm
        checks(label + '-inputs-' + side + '.txt', mvm)
for phase in ('clippy', 'check', 'bdd', 'footprint'):
    assert text('retention-full1-' + phase + '-time.txt').rstrip().endswith('Exit status: 0')
assert 'measurement only' in text('retention-full1-footprint.txt')
assert 'check-all: 86 gate(s) clean' in text('retention-full2-policy.txt')
assert text('retention-full2-tests-time.txt').rstrip().endswith('Exit status: 0')
prerequisites = data('retention-full2-prerequisite-source-gates.json')
assert prerequisites['source'] == source and prerequisites['prior_original_exit'] == 1
assert prerequisites['prior_source_guards_match'] is True

compiled = data('retention-full2-executables-before.json')
assert len(compiled) == 286 and all(r['fresh'] is False and r['prior_root_occurrences'] == 0 for r in compiled)
assert hashes('retention-full2-executables-before.sha256') == {r['path']: r['sha256'] for r in compiled}
checks('retention-full2-executables-after.txt', {r['path'] for r in compiled})
running = re.findall(r'^     Running .* \((/[^()\n]+)\)$', text('retention-full2-tests.txt'), re.M)
assert len(running) == 286 and set(running) == {r['path'] for r in compiled}
summary = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
groups = []
current = None
for line in text('retention-full2-tests.txt').splitlines():
    if line.startswith('     Running ') or line.startswith('   Doc-tests '):
        if current:
            groups.append(current)
        current = dict(kind='binary' if line.startswith('     Running ') else 'doc', results=[])
    elif (match := summary.match(line)) and current:
        current['results'].append(tuple(map(int, match.groups()[1:])))
if current:
    groups.append(current)
assert all(g['results'] for g in groups)
for kind, passes, ignored in (('binary', 21890, 72), ('doc', 27, 8)):
    selected = [g for g in groups if g['kind'] == kind]
    rows = [r for g in selected for r in (g['results'][-1:] if kind == 'binary' else g['results'])]
    assert tuple(sum(r[i] for r in rows) for i in range(3)) == (passes, 0, ignored)

assert data('retention-sdk1-outcome.json')['original_exit'] == 101
assert 'test result: FAILED. 174 passed; 1 failed; 1 ignored;' in text('retention-sdk1-live-tests.txt')
assert 'startup requires a local disk-backed filesystem' in text('retention-sdk1-live-tests.txt')
assert text('retention-startup1-filesystem.txt').startswith('ext4 ')
assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text('retention-startup1-tests.txt')
assert hashes('retention-sdk1-live-executable-before.sha256') == hashes('retention-startup1-executable-before.sha256')
checks('retention-startup1-executable-after.txt', hashes('retention-startup1-executable-before.sha256'))
for label in ('retention-sdk2', 'retention-paired1', 'retention-sdk-metadata1'):
    clean(label + '-outcome.json')
for side in ('before', 'after'):
    checks('retention-sdk2-sdk-inputs-' + side + '.txt', sdk)
    assert hashes('retention-sdk2-sdk-file-set-' + side + '.sha256') == sdk
    checks('retention-sdk2-mvm-inputs-' + side + '.txt', mvm)
    checks('retention-sdk2-git-inputs-' + side + '.txt', mvm)
    checks('retention-paired1-inputs-' + side + '.txt', paired)
    assert hashes('retention-paired1-file-set-' + side + '.sha256') == paired
    assert hashes('retention-paired1-fixtures-' + side + '.sha256') == fixtures
    checks('retention-paired1-sdk-inputs-' + side + '.txt', sdk)
    checks('retention-paired1-mvm-inputs-' + side + '.txt', mvm)
    if side == 'after':
        assert hashes('retention-paired1-sdk-file-set-after.sha256') == sdk
        assert hashes('retention-paired1-mvm-file-set-after.sha256') == mvm
git_before = hashes('retention-sdk2-git-file-set-before.sha256')
assert set(git_before) - set(mvm) == {'./.cargo-ok'}
assert all(git_before[p] == h for p, h in mvm.items())
assert git_before['./.cargo-ok'] == hashlib.sha256(b'').hexdigest()
assert hashes('retention-sdk2-git-file-set-after.sha256') == git_before
assert hashes('retention-sdk2-mvm-file-set-after.sha256') == mvm
assert 'test result: ok. 175 passed; 0 failed; 1 ignored;' in text('retention-sdk2-live-tests.txt')
assert hashes('retention-sdk2-live-executable-before.sha256') == hashes('retention-startup1-executable-before.sha256')
for label in ('live', 'journal'):
    checks('retention-sdk2-' + label + '-executable-after.txt', hashes('retention-sdk2-' + label + '-executable-before.sha256'))
for label, passes in (('owner', 29), ('supervisor', 74)):
    rows = data('retention-sdk2-' + label + '-executables-before.json')
    checks('retention-sdk2-' + label + '-executables-after.txt', {r['path'] for r in rows})
    audit = data('retention-sdk2-' + label + '-runtime.json')
    assert audit['passed'] == passes and audit['ignored'] == 0 and audit['exact_compiler_set'] is True
assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text('retention-sdk2-journal.txt')
for label in ('live', 'adapter', 'witness', 'journal'):
    rows = data('retention-sdk2-' + label + '-profiles.json')
    assert {r['name'] for r in rows} == {'mvm-core', 'mvm-crypto'}
    for row in rows:
        assert row['source'] == f'git+https://github.com/Casey-allard/mvm?rev={source}#{source}'
        profile = row['profile']
        assert profile['opt_level'] == '2' and profile['debug_assertions'] and profile['overflow_checks']
    checks('retention-sdk2-' + label + '-profile-artifacts-after.txt', hashes('retention-sdk2-' + label + '-profiles.sha256'))
paired_elfs = data('retention-paired1-executables-before.json')
assert len(paired_elfs) == 4 and all(r['fresh'] is False for r in paired_elfs)
checks('retention-paired1-executables-after.txt', {r['path'] for r in paired_elfs})
actual = re.findall(r'^     Running .*? \(([^)]+)\)$', text('retention-paired1-tests.txt'), re.M)
assert len(actual) == 4 and set(actual) == {r['path'] for r in paired_elfs}
rows = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text('retention-paired1-tests.txt'), re.M)
assert tuple(sum(int(r[i]) for r in rows) for i in range(3)) == (18, 0, 1)
assert 'emit_first_object_call_for_guest_component ... ignored' in text('retention-paired1-tests.txt')
for name, value in records.items():
    if name.startswith('retention-graphs/') and name.endswith('.json') and not name.endswith('-summary.json') and not name.endswith('/summary.json'):
        assert value == records['baseline/late-unlink-cc2-graphs/' + name.split('/')[-1]]
for name in ('retention-current-executables-receipt.json', 'retention-sdk-executables-receipt.json', 'retention-paired-executables-receipt.json'):
    receipt = data(name)
    assert receipt['all_members_replayed'] and receipt['all_originals_posthashed']
    assert receipt['historical_79_lost_preimages_not_recovered'] is True
assert text('retention-sdk2-spiralcore-source.txt').strip() == sc
assert text('retention-sdk2-mvm-source.txt').strip() == source
print(json.dumps(dict(records=len(records),binary_passed=21890,doctest_passed=27,
                     source_files=4758,compiler_executed_ELFs=286,SDK='175/29/74/1',
                     paired_passed=18,ignored_paired=1,graphs_equal=9,
                     scope='retained source/component evidence; no archived code or binary executed')))
