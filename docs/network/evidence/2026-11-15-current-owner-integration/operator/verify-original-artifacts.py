import json, hashlib, zipfile, pathlib, re
root=pathlib.Path('/tmp/sc-current-owner-evidence/native')
rows=json.load(open('/tmp/sc-current-owner-artifacts.json'))
for row in rows:
 p=root/(row['name']+'.zip'); raw=p.read_bytes()
 assert len(raw)==row['size'],p
 assert 'sha256:'+hashlib.sha256(raw).hexdigest()==row['digest'],p
 with zipfile.ZipFile(p) as z:
  assert len(z.namelist())==len(set(z.namelist()))
  for n in z.namelist():
   assert not n.startswith('/') and '..' not in pathlib.PurePosixPath(n).parts,n
  verified=0
  for n in z.namelist():
   if pathlib.PurePosixPath(n).name=='SHA256SUMS':
    for line in z.read(n).decode().splitlines():
     m=re.match(r'^([a-fA-F0-9]{64})\s+\*?(.+?)\s*$',line)
     assert m,(n,line)
     name=m[2].replace('\\','/').removeprefix('./')
     matches=[v for v in z.namelist() if v==name or v.endswith('/'+name)]
     assert len(matches)==1,(n,name,matches)
     assert hashlib.sha256(z.read(matches[0])).hexdigest()==m[1].lower(),(p,name)
     verified+=1
  target=root/row['name']; target.mkdir(exist_ok=True); z.extractall(target)
  row['members']=len(z.namelist()); row['embedded_verified']=verified
json.dump(rows,open('/tmp/sc-current-owner-evidence/native/inventory.json','w'),indent=2)
print(json.dumps({'archives':len(rows),'bytes':sum(x['size'] for x in rows),'members':sum(x['members'] for x in rows),'embedded_verified':sum(x['embedded_verified'] for x in rows)}))
