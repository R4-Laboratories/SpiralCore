#!/usr/bin/env python3
"""Verify retained records without running their code or extracting files."""
from pathlib import Path
import hashlib,json,re,subprocess,tarfile

root=Path(__file__).resolve().parent
identity=json.loads((root/'archive-identity.json').read_text())
archive=root/identity['file']
assert archive.stat().st_size==identity['bytes']==1921298
assert hashlib.sha256(archive.read_bytes()).hexdigest()==identity['sha256']
index=json.loads((root/'records-index.json').read_text())['files']
expected={r['path']:r for r in index}
assert len(index)==len(expected)==identity['members']==560
assert sum(r['bytes'] for r in index)==identity['uncompressed_file_bytes']==95202737
records={}
process=subprocess.Popen(['zstd','-q','-d','-c',str(archive)],stdout=subprocess.PIPE)
with tarfile.open(fileobj=process.stdout,mode='r|') as tar:
    for member in tar:
        assert member.isfile() and member.name in expected and member.name not in records
        assert not member.name.startswith('/') and '..' not in Path(member.name).parts
        row=expected[member.name]
        assert member.size==row['bytes']<25_000_000
        value=tar.extractfile(member).read()
        assert hashlib.sha256(value).hexdigest()==row['sha256']
        assert not value.startswith(b'\x7fELF')
        records[member.name]=value
assert process.wait()==0 and set(records)==set(expected)
def raw(name): return records[name]
def data(name): return json.loads(raw(name))
def text(name): return raw(name).decode()
def checks(name,count,paths=None):
    rows=text(name).splitlines()
    assert len(rows)==count and all(s.endswith(': OK') for s in rows),name
    if paths is not None: assert set(rows)=={p+': OK' for p in paths},name
def hashes(name):
    rows=[line.split('  ',1) for line in text(name).splitlines()]
    result={p.removeprefix('./'):h for h,p in rows}
    assert len(result)==len(rows)
    return result

for prefix in ('late-unlink-controls1','late-unlink-controls2','late-unlink-sdk-metadata1','late-unlink-sdk-metadata2','late-unlink-sdk1','late-unlink-sdk2','late-unlink-paired2'):
    assert all(v==0 for v in data(prefix+'-outcome.json').values()),prefix
for prefix in ('late-unlink-full1','late-unlink-full2'):
    assert data(prefix+'-outcome.json')==dict(original_exit=1,source_hash_exit=0,file_enumeration_exit=0,file_set_comparison_exit=0,final_exit=1)
    frozen=raw(prefix+'-frozen-inputs.sha256')
    assert len(frozen.splitlines())==4757
    assert frozen==raw(prefix+'-file-set-before.sha256')==raw(prefix+'-file-set-after.sha256')
    checks(prefix+'-inputs-before.txt',4757);checks(prefix+'-inputs-after.txt',4757)
    assert text(prefix+'-tests-time.txt').rstrip().endswith('Exit status: 0')
    assert 'check-all: 86 gate(s) clean' in text(prefix+'-policy.txt')
full=data('late-unlink-full2-test-counts.json')
assert full['source']=='cc2ed208a7b1adde7fa1f075bbfff2428f911e18'
summary=re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
groups=[];current=None
for line in text('late-unlink-full2-tests.txt').splitlines():
    if line.startswith('     Running ') or line.startswith('   Doc-tests '):
        if current: groups.append(current)
        current={'kind':'binary' if line.startswith('     Running ') else 'doc','summaries':[]}
    else:
        match=summary.match(line)
        if match and current: current['summaries'].append(tuple(map(int,match.groups()[1:])))
if current: groups.append(current)
assert all(g['summaries'] for g in groups)
for kind,passes,ignores in (('binary',21884,72),('doc',27,8)):
    selected=[g for g in groups if g['kind']==kind]
    rows=[r for g in selected for r in (g['summaries'][-1:] if kind=='binary' else g['summaries'])]
    assert (sum(r[0] for r in rows),sum(r[1] for r in rows),sum(r[2] for r in rows))==(passes,0,ignores)
    assert full['summary'][kind]['passed']==passes and full['summary'][kind]['ignored']==ignores
current=data('late-unlink-full2-executables-before.json')
assert len(current)==286 and all(r['fresh'] is False for r in current)
by={r['path']:r for r in current};assert len(by)==286
checks('late-unlink-full2-executables-after.txt',286,by)
running=re.findall(r'^     Running .* \((/[^()\n]+)\)$',text('late-unlink-full2-tests.txt'),re.M)
assert len(running)==286 and set(running)==set(by)
assert data('late-unlink-full2-runtime-executable-set.json')==dict(expected=286,running_lines=286,unique_executed=286,exact_match=True)
old={r['path']:r for r in data('late-unlink-full1-executables-before.json')}
prior=text('late-unlink-full2-prior-executables-after.txt').splitlines()
failed={r.removesuffix(': FAILED') for r in prior if r.endswith(': FAILED')}
unchanged={r.removesuffix(': OK') for r in prior if r.endswith(': OK')}
loss=data('late-unlink-full2-preservation-failure.json')
mapping=loss['overwritten_prior_executables']
assert len(prior)==286 and len(failed)==len(mapping)==len({r['path'] for r in mapping})==79 and len(unchanged)==207
assert failed.isdisjoint(unchanged) and failed|unchanged==set(old) and {r['path'] for r in mapping}==failed
for row in mapping:
    assert row['old_sha256']==old[row['path']]['sha256'] and row['current_sha256']==by[row['path']]['sha256']
    assert row['old_sha256']!=row['current_sha256'] and row['current_recompiled']
proof=data('late-unlink-full2-current-source-proof.json')
assert proof['original_full_outcome']==data('late-unlink-full2-outcome.json')
assert proof['current_tests_and_required_source_gates_pass'] and proof['historical_preservation_failed'] and proof['loss_not_repaired'] and proof['tests_rerun'] is False
assert proof['older_original_bytes_lost']==79
for number,count in ((1,596),(2,9),(3,4)):
    receipt=data(f'late-unlink-elf-archive{number}-receipt.json')
    assert receipt['files']==count and receipt['all_members_replayed_and_matched'] and receipt['current_file_posthashes_matched']
classification=data('late-unlink-elf-archive1-classification.json')
assert len(classification['files'])==596 and sum(r['current_cc2_full'] for r in classification['files'])==286
assert classification['known_full2_historical_preimages_lost']==mapping
for prefix in ('late-unlink-sdk1','late-unlink-sdk2'):
    frozen=raw(prefix+'-frozen-inputs.sha256');assert len(frozen.splitlines())==490
    assert frozen==raw(prefix+'-sdk-file-set-before.sha256')==raw(prefix+'-sdk-file-set-after.sha256')
    for phase in ('before','after'):
        checks(prefix+'-sdk-inputs-'+phase+'.txt',490)
        checks(prefix+'-mvm-inputs-'+phase+'.txt',4757)
        checks(prefix+'-git-inputs-'+phase+'.txt',4757)
    assert raw(prefix+'-git-file-set-before.sha256')==raw(prefix+'-git-file-set-after.sha256')
    assert all(v==0 for v in data(prefix+'-git-source-outcome.json').values())
result=re.compile(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',re.M)
assert result.findall(text('late-unlink-sdk2-live-tests.txt'))[-1]==('175','0','1')
for label,count,targets in (('owner',29,1),('supervisor',74,6)):
    prefix='late-unlink-sdk2-'+label
    runtime=data(prefix+'-runtime.json');artifacts=data(prefix+'-executables-before.json')
    assert runtime['passed']==count and runtime['ignored']==0 and runtime['exact_compiler_set']
    assert len(artifacts)==len(runtime['actual_paths'])==targets and {r['path'] for r in artifacts}==set(runtime['actual_paths'])
    assert all(r['fresh'] is False for r in artifacts)
    checks(prefix+'-executables-after.txt',targets,{r['path'] for r in artifacts})
for label in ('live','adapter','witness','journal'):
    profiles=data('late-unlink-sdk2-'+label+'-profiles.json')
    assert {r['name'] for r in profiles}=={'mvm-core','mvm-crypto'}
    build=[json.loads(s) for s in text('late-unlink-sdk2-'+label+'-build.jsonl').splitlines()]
    for r in profiles:
        selected=[x for x in build if x.get('reason')=='compiler-artifact' and x.get('package_id')==r['package_id'] and x['target']['kind']==['lib'] and not x['profile']['test']]
        assert len(selected)==1 and selected[0]['profile']==r['profile'] and selected[0]['fresh']==r['fresh']
        assert r['source']=='git+https://github.com/Casey-allard/mvm?rev=cc2ed208a7b1adde7fa1f075bbfff2428f911e18#cc2ed208a7b1adde7fa1f075bbfff2428f911e18'
        assert r['profile']['opt_level']=='2' and r['profile']['debug_assertions'] and r['profile']['overflow_checks']
    checks('late-unlink-sdk2-'+label+'-profile-artifacts-after.txt',4)
paired=data('late-unlink-paired2-executables-before.json')
frozen=raw('late-unlink-paired2-frozen-inputs.sha256')
assert len(frozen.splitlines())==17 and frozen==raw('late-unlink-paired2-file-set-before.sha256')==raw('late-unlink-paired2-file-set-after.sha256')
for phase in ('before','after'):
    checks('late-unlink-paired2-inputs-'+phase+'.txt',17)
    checks('late-unlink-paired2-sdk-inputs-'+phase+'.txt',490)
    checks('late-unlink-paired2-mvm-inputs-'+phase+'.txt',4757)
assert raw('late-unlink-paired2-sdk-file-set-after.sha256')==raw('late-unlink-sdk2-frozen-inputs.sha256')
assert raw('late-unlink-paired2-mvm-file-set-after.sha256')==raw('late-unlink-full2-frozen-inputs.sha256')
assert len(raw('late-unlink-paired2-fixtures-before.sha256').splitlines())==5
assert raw('late-unlink-paired2-fixtures-before.sha256')==raw('late-unlink-paired2-fixtures-after.sha256')
assert len(paired)==4 and all(r['fresh'] is False for r in paired)
assert data('late-unlink-paired2-runtime-executable-set.json')==dict(compiled_targets=4,actual_running_targets=4,exact_match=True)
checks('late-unlink-paired2-executables-after.txt',4,{r['path'] for r in paired})
cases=result.findall(text('late-unlink-paired2-tests.txt'))
assert tuple(sum(int(r[i]) for r in cases) for i in range(3))==(18,0,1)
for label in ('live','adapter','journal','owner','supervisor','witness','guest','mvm'):
    assert data('late-unlink-graphs/'+label+'-after.json')==data('late-unlink-cc2-graphs/'+label+'.json')
assert data('late-unlink-cc2-graphs/live.json')==data('late-unlink-cc2-graphs/live-canonical-linux.json')
cleanup=[data('sc-cc2-cache-cleanup.json'),data('sc-cc2-cache-cleanup2.json')]
assert sum(r['deleted_files'] for r in cleanup)==10399
assert sum(r['deleted_apparent_bytes'] for r in cleanup)==37730446048
for r in cleanup:
    assert r['result']=='success' and all(p['exit']==1 and not p['stdout'] for p in r['lsof'])
    assert all(Path(p['path']).suffix in {'.rlib','.rmeta'} for p in r['deleted'])
identities=json.loads((root/'source-identities.json').read_text())
assert {identities[k]['tree'] for k in ('reviewed-head','ci-merge','release')}=={'ff95d48508419daa17f9f6bace0e8a6dea15b162'}
assert identities['release']['sha']=='00ec196bc70b013aa470c4b7a20affaa05c634b1'
assert identities['mvm-source']['sha']=='cc2ed208a7b1adde7fa1f075bbfff2428f911e18' and identities['mvm-source']['tree']=='52e3463526c94a5941a16fa187158fd630c258bd'
print('PASS: 560 captured records; current-source/SDK/paired receipts; original failures and 79 lost historical preimages remain explicit; no archived code executed')
