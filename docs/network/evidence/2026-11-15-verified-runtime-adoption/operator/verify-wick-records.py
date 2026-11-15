import hashlib
import json
import pathlib
import sys
import zipfile

archive = pathlib.Path(sys.argv[1])
receipt = json.loads(pathlib.Path(sys.argv[2]).read_text())
data = archive.read_bytes()
assert len(data) == receipt['zip_bytes']
assert hashlib.sha256(data).hexdigest() == receipt['sha256']
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    records = json.loads(z.read('record-index.json'))['records']
    assert len(records) == receipt['records']
    assert len(z.namelist()) == len(records) + 1
    assert len(z.namelist()) == len(set(z.namelist()))
    for row in records:
        name = pathlib.PurePosixPath(row['name'])
        assert not name.is_absolute() and '..' not in name.parts
        raw = z.read(row['name'])
        assert len(raw) == row['bytes']
        assert hashlib.sha256(raw).hexdigest() == row['sha256']
    assert sum(row['bytes'] for row in records) == receipt['record_bytes']
print(json.dumps({'verified_records': len(records), 'record_bytes': receipt['record_bytes'],
                  'zip_bytes': len(data), 'sha256': receipt['sha256']}))
