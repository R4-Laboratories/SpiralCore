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
assert archive.stat().st_size == identity['bytes'] == 1352013
assert hashlib.sha256(archive.read_bytes()).hexdigest() == identity['sha256']
index = json.loads((root / 'records-index.json').read_text())['files']
expected = {r['path']: r for r in index}
assert len(index) == len(expected) == identity['members'] == 478
assert sum(r['bytes'] for r in index) == identity['uncompressed_file_bytes'] == 74612613
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


source = 'f2ddbcbf1039405698e1ac0dcb16e29a5bf3b676'
sc = '698c1fc5fe64ce0c82b5f1948ed740759853a19d'
mvm = hashes('sc-parent-sync-green4-frozen-inputs.sha256')
sdk = hashes('sc-parent-sync-sdk-frozen-inputs.sha256')
paired = hashes('baseline/sc-retention-paired-frozen-inputs.sha256')
fixtures = hashes('baseline/sc-retention-paired-fixtures.sha256')
assert tuple(map(len, (mvm, sdk, paired, fixtures))) == (4758, 490, 17, 5)

# Preserve failed source-specific gates and the earlier test body.
assert data('parent-sync-full1-outcome.json') == dict(
    controller_exit=1, source_hash_exit=0, enumeration_exit=0, file_set_exit=0, final_exit=1)
assert text('parent-sync-full1-source.txt').strip() == 'fa806a94890e1200358c00fc97ce06504ad20de7'
assert 'expected 0 unexplained public sibling pairs, found 1' in text('parent-sync-full1-policy.txt')
assert 'pending_installed_store_with_parent' in text('parent-sync-full1-policy.txt')
assert data('parent-sync-green3-outcome.json') == dict(
    controller_exit=1, source_hash_exit=0, enumeration_exit=0, file_set_exit=0, final_exit=1)
assert 'prepare_store_fixture(|path|' in text('parent-sync-green3-fmt.txt')
for label in ('parent-sync-green1', 'parent-sync-green2', 'parent-sync-green4',
              'parent-sync-full2', 'parent-sync-sdk-metadata1', 'parent-sync-sdk1',
              'parent-sync-paired1'):
    clean(label + '-outcome.json')
assert hashlib.sha256(records['sc-parent-sync-test.patch']).hexdigest() == 'bc461f9cae2d0475a7fd893cca294175839326a1484eb267fb96729e863aac67'
assert hashlib.sha256(records['sc-parent-sync-green4-test.patch']).hexdigest() == '18ac65ee34177e18759516832331438bab26fc83b19739b405656e9683f3e25f'

assert text('parent-sync-full2-source.txt').strip() == source
for side in ('before', 'after'):
    assert hashes('parent-sync-full2-file-set-' + side + '.sha256') == mvm
    checks('parent-sync-full2-inputs-' + side + '.txt', mvm)
for phase in ('public-names', 'clippy', 'check', 'bdd', 'footprint', 'policy', 'build', 'tests'):
    assert text('parent-sync-full2-' + phase + '-time.txt').rstrip().endswith('Exit status: 0')
assert 'measurement only' in text('parent-sync-full2-footprint.txt')
assert 'check-all: 86 gate(s) clean' in text('parent-sync-full2-policy.txt')
compiled = data('parent-sync-full2-executables-before.json')
prior = {r['path']: r for r in data('baseline/retention-full2-executables-before.json')}
assert len(compiled) == len(prior) == 286
assert all(r['fresh'] is False for r in prior.values())
checks('baseline/retention-full2-executables-after.txt', prior)
assert len(re.findall(r'^     Running .* \((/[^()\n]+)\)$', text('baseline/retention-full2-tests.txt'), re.M)) == 286
assert sum(r['fresh'] for r in compiled) == 26
assert all(r['prior_root_occurrences'] == 0 for r in compiled)
for row in compiled:
    if row['fresh']:
        assert row['same_root_reuse_verified'] and prior[row['path']]['sha256'] == row['sha256']
assert hashes('parent-sync-full2-executables-before.sha256') == {r['path']: r['sha256'] for r in compiled}
for name in ('after', 'finally'):
    checks('parent-sync-full2-executables-' + name + '.txt', {r['path'] for r in compiled})
running = re.findall(r'^     Running .* \((/[^()\n]+)\)$', text('parent-sync-full2-tests.txt'), re.M)
assert len(running) == 286 and set(running) == {r['path'] for r in compiled}
summary = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
groups, current = [], None
for line in text('parent-sync-full2-tests.txt').splitlines():
    if line.startswith('     Running ') or line.startswith('   Doc-tests '):
        if current:
            groups.append(current)
        current = dict(kind='binary' if line.startswith('     Running ') else 'doc', results=[])
    elif (match := summary.match(line)) and current:
        current['results'].append(tuple(map(int, match.groups()[1:])))
if current:
    groups.append(current)
assert all(g['results'] for g in groups)
for kind, passes, ignored in (('binary', 21891, 72), ('doc', 27, 8)):
    selected = [g for g in groups if g['kind'] == kind]
    rows = [r for g in selected for r in (g['results'][-1:] if kind == 'binary' else g['results'])]
    assert tuple(sum(r[i] for r in rows) for i in range(3)) == (passes, 0, ignored)

for side in ('before', 'after'):
    for owner in ('sdk', 'mvm', 'git'):
        checks('parent-sync-sdk1-' + owner + '-inputs-' + side + '.txt', sdk if owner == 'sdk' else mvm)
    assert hashes('parent-sync-sdk1-sdk-file-set-' + side + '.sha256') == sdk
    checks('parent-sync-paired1-inputs-' + side + '.txt', paired)
    assert hashes('parent-sync-paired1-file-set-' + side + '.sha256') == paired
    assert hashes('parent-sync-paired1-fixtures-' + side + '.sha256') == fixtures
    checks('parent-sync-paired1-sdk-inputs-' + side + '.txt', sdk)
    checks('parent-sync-paired1-mvm-inputs-' + side + '.txt', mvm)
assert hashes('parent-sync-sdk1-mvm-file-set-after.sha256') == mvm
git = hashes('parent-sync-sdk1-git-file-set-before.sha256')
assert set(git) - set(mvm) == {'./.cargo-ok'}
assert git['./.cargo-ok'] == hashlib.sha256(b'').hexdigest()
assert all(git[p] == h for p, h in mvm.items())
assert hashes('parent-sync-sdk1-git-file-set-after.sha256') == git
assert hashes('parent-sync-paired1-sdk-file-set-after.sha256') == sdk
assert hashes('parent-sync-paired1-mvm-file-set-after.sha256') == mvm
assert 'test result: ok. 175 passed; 0 failed; 1 ignored;' in text('parent-sync-sdk1-live-tests.txt')
assert data('parent-sync-sdk1-live-selected-artifact.json')['fresh'] is False
for label in ('live', 'journal'):
    checks('parent-sync-sdk1-' + label + '-executable-after.txt', hashes('parent-sync-sdk1-' + label + '-executable-before.sha256'))
for label, passes in (('owner', 29), ('supervisor', 74)):
    rows = data('parent-sync-sdk1-' + label + '-executables-before.json')
    checks('parent-sync-sdk1-' + label + '-executables-after.txt', {r['path'] for r in rows})
    audit = data('parent-sync-sdk1-' + label + '-runtime.json')
    assert audit['passed'] == passes and audit['ignored'] == 0 and audit['exact_compiler_set'] is True
assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in text('parent-sync-sdk1-journal.txt')
for label in ('live', 'adapter', 'witness', 'journal'):
    rows = data('parent-sync-sdk1-' + label + '-profiles.json')
    meta = data('parent-sync-sdk1-' + label + '-metadata.json')
    build = [json.loads(line) for line in text('parent-sync-sdk1-' + label + '-build.jsonl').splitlines()]
    assert {r['name'] for r in rows} == {'mvm-core', 'mvm-crypto'}
    for row in rows:
        assert row['source'] == f'git+https://github.com/Casey-allard/mvm?rev={source}#{source}'
        package = [p for p in meta['packages'] if p['name'] == row['name'] and p['source'] == row['source']]
        assert len(package) == 1 and row['package_id'] == package[0]['id']
        actual = [r for r in build if r.get('reason') == 'compiler-artifact' and r.get('package_id') == row['package_id'] and r['target']['kind'] == ['lib'] and not r['profile']['test']]
        assert len(actual) == 1 and actual[0]['profile'] == row['profile']
        assert set(actual[0]['filenames']) == {f['path'] for f in row['files']}
        p = row['profile']
        assert p['opt_level'] == '2' and p['debug_assertions'] and p['overflow_checks']
    checks('parent-sync-sdk1-' + label + '-profile-artifacts-after.txt', hashes('parent-sync-sdk1-' + label + '-profiles.sha256'))
paired_elfs = data('parent-sync-paired1-executables-before.json')
old_pair = {r['path']: r for r in data('baseline/retention-paired1-executables-before.json')}
assert len(paired_elfs) == len(old_pair) == 4 and all(not r['fresh'] for r in old_pair.values())
checks('baseline/retention-paired1-executables-after.txt', old_pair)
for row in paired_elfs:
    assert row['fresh'] and row['sha256'] == old_pair[row['path']]['sha256']
checks('parent-sync-paired1-executables-after.txt', {r['path'] for r in paired_elfs})
actual = re.findall(r'^     Running .*? \(([^)]+)\)$', text('parent-sync-paired1-tests.txt'), re.M)
assert len(actual) == 4 and set(actual) == {r['path'] for r in paired_elfs}
rows = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text('parent-sync-paired1-tests.txt'), re.M)
assert tuple(sum(int(r[i]) for r in rows) for i in range(3)) == (18, 0, 1)
assert 'emit_first_object_call_for_guest_component ... ignored' in text('parent-sync-paired1-tests.txt')
profiles = data('parent-sync-paired1-profiles.json')
assert profiles['source'] == source and len(profiles['path_packages']) == 6
assert {r['name'] for r in profiles['records']} == {'mvm-core', 'mvm-crypto'}
for row in profiles['records']:
    p = row['profile']
    assert p['opt_level'] == '0' and p['debug_assertions'] and p['overflow_checks']
checks('parent-sync-paired1-profile-artifacts-after.txt', hashes('parent-sync-paired1-profiles.sha256'))
graphs = 0
for name, value in records.items():
    if name.startswith('parent-sync-graphs/') and name.endswith('.json') and not name.endswith('-summary.json') and not name.endswith('/summary.json'):
        assert value == records['baseline/retention-graphs/' + name.split('/')[-1]]
        graphs += 1
assert graphs == 9
for name in ('parent-sync-current-executables-receipt.json', 'parent-sync-sdk-executables-receipt.json', 'baseline/retention-paired-executables-receipt.json'):
    receipt = data(name)
    assert receipt['all_members_replayed'] and receipt['all_originals_posthashed']
    assert receipt['historical_79_lost_preimages_not_recovered'] is True
assert text('parent-sync-sdk1-spiralcore-source.txt').strip() == sc
assert text('parent-sync-sdk1-mvm-source.txt').strip() == source
print(json.dumps(dict(records=len(records), binary_passed=21891, doctest_passed=27,
                     source_files=4758, compiler_executed_ELFs=286, recompiled=260,
                     verified_reuse=26, SDK='175/29/74/1', paired_passed=18,
                     paired_reused_ELFs=4, ignored_paired=1, graphs_equal=9,
                     scope='retained source/component records; no archived code or binary executed')))


