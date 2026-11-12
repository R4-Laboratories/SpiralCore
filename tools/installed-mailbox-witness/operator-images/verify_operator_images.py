"""Independently inspect the completed MVM ext4 image and dm-verity tree."""
import hashlib
import json
import struct
import subprocess
import sys
from pathlib import Path

if not __debug__:
    raise SystemExit("Run the image verifier without Python optimization so every check executes")

def sha(data):
    return hashlib.sha256(data).hexdigest()

def debug(image, op):
    p = subprocess.run(["/usr/sbin/debugfs", "-R", op, image], capture_output=True, check=True)
    return p.stdout, p.stderr

def contents(image, name):
    data, err = debug(image, f"cat {name}")
    assert b"File not found" not in err and data, (name, err)
    return data

def verify_verity(raw, image_dir):
    sb = 1024
    assert raw[sb + 56:sb + 58] == b"\x53\xef"
    data_block = 1024 << struct.unpack_from("<I", raw, sb + 24)[0]
    assert data_block == 4096 and len(raw) % data_block == 0
    salt = bytes(32)
    def digest(block):
        return hashlib.sha256(salt + block).digest()
    nodes = [digest(raw[i:i+data_block]) for i in range(0, len(raw), data_block)]
    levels = []
    while True:
        layer = b"".join(nodes)
        layer += bytes((-len(layer)) % 4096)
        levels.append(layer)
        nodes = [digest(layer[i:i+4096]) for i in range(0, len(layer), 4096)]
        if len(nodes) == 1:
            break
    tree = (image_dir / "rootfs.verity").read_bytes()
    root = (image_dir / "rootfs.roothash").read_text().strip()
    assert tree == b"".join(reversed(levels))
    assert root == nodes[0].hex()
    return root

work, preparer_json, manifest = map(Path,sys.argv[1:])
runtime = json.loads(preparer_json.read_bytes())
inputs = {k:v for v,k in (line.split() for line in manifest.read_text().splitlines())}
programs = {"a":"spiralcore-mailbox-producer","b":"spiralcore-cast-recipient"}
report = []
for role, name in programs.items():
    d = work/role/"image"
    image = d/"rootfs.ext4"
    raw = image.read_bytes()
    root = verify_verity(raw,d)
    executable="/usr/local/bin/"+name
    other=next(q for q in programs.values() if q != name)
    assert b"File not found" in debug(image,"stat /usr/local/bin/"+other)[1]
    assert b"File not found" in debug(image,"stat /etc/mvm/untrusted-marker")[1]
    marker=contents(image,"/etc/mvm/entrypoint")
    wrapper="/usr/lib/mvm/wrappers/oci-entrypoint"
    assert marker == (wrapper+"\n").encode()
    imgcfg = json.loads(contents(image,"/etc/mvm/image-runtime.json"))
    assert imgcfg == {"argv":[executable],"env":[],"working_dir":"/"}
    worker=json.loads(contents(image,"/etc/mvm/runtime.json"))
    assert worker == {"language":"rust","module":"cast","function":"main","format":"binary","source_path":executable,"concurrency":{"kind":"warm_process","max_calls_per_worker":100,"max_rss_mb":64,"pool_size":1,"in_process":"serial","max_queue_depth":1}}
    assert b"UNTRUSTED FIXTURE CONTENT" not in raw
    assert contents(image,"/etc/mvm/variant") == b"prod\n"
    verb=json.loads(contents(image,"/etc/mvm/verb-trust.json"))
    assert verb.get("require_grant") is True, verb
    binary=contents(image,executable)
    assert sha(binary) == inputs[role+executable] and binary[:4] == b"\x7fELF"
    wrapper_sha=sha(contents(image,wrapper))
    assert wrapper_sha == runtime["runtime_manifest"]["files"]["x86_64/bin/mvm-oci-entrypoint"]
    meta=json.loads((d/"mvm-meta.json").read_text())
    assert meta["sealed"] and meta["overlayAware"] and meta["runtimeLean"] and meta["entrypointArgv"] == [executable]
    report.append(dict(role=role,image_bytes=len(raw),image_sha256=sha(raw),verity_root=root,verity_tree_sha256=sha((d/"rootfs.verity").read_bytes()),workload_path=executable,workload_bytes=len(binary),workload_sha256=sha(binary),wrapper_sha256=wrapper_sha,worker=worker["concurrency"],entrypoint=wrapper,require_grant=verb["require_grant"],negative_control="scrubbed",verify="debugfs readback and independent full dm-verity v1 tree recomputation"))
print(json.dumps(report, indent=2))
