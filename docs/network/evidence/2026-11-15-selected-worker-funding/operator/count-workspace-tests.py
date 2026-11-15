from pathlib import Path
import re,json,sys
raw=Path(sys.argv[1]).read_bytes()
lines=raw.decode().splitlines();groups=[];current=None
summary=re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;')
for line in lines:
 if line.startswith('     Running '):
  if current:groups.append(current)
  current={'kind':'binary','target':line.strip(),'summaries':[]}
 elif line.startswith('   Doc-tests '):
  if current:groups.append(current)
  current={'kind':'doc','target':line.strip(),'summaries':[]}
 else:
  m=summary.match(line)
  if m and current:current['summaries'].append({'status':m[1],'passed':int(m[2]),'failed':int(m[3]),'ignored':int(m[4])})
if current:groups.append(current)
assert all(x['summaries'] for x in groups),[x['target'] for x in groups if not x['summaries']]
result={}
for kind in ('binary','doc'):
 selected=[x for x in groups if x['kind']==kind];rows=[s for x in selected for s in (x['summaries'][-1:] if kind=='binary' else x['summaries'])]
 result[kind]={'sections':len(selected),'outer_summaries':len(rows),'passed':sum(x['passed'] for x in rows),'failed':sum(x['failed'] for x in rows),'ignored':sum(x['ignored'] for x in rows),'excluded_nested_summaries':sum(len(x['summaries'])-1 for x in selected) if kind=='binary' else 0}
if len(sys.argv)>2:
 Path(sys.argv[2]).write_text(json.dumps({'source':'8345cb682ecbc755c7312ba7a5a28c641aa667d0','summary':result,'groups':groups},indent=2)+'\n')
print(json.dumps(result))
