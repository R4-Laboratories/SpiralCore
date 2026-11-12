"""Probe the mounted, read-only ext4 as the unprivileged guest service identity."""
import hashlib
import json
import os
import stat
import sys
from pathlib import Path

def require(condition, message):
    if not condition:
        raise RuntimeError(message)

root, role, program_digest, wrapper_digest = sys.argv[1:]
mount = Path(root)
require(role in {"a", "b"}, "invalid role")
require(mount.is_mount(), "probe requires the actual mounted filesystem")
require(os.statvfs(mount).f_flag & os.ST_RDONLY, "probe requires a read-only filesystem")
require(os.getresuid() == (1000, 1000, 1000), "unexpected service uid")
require(os.getresgid() == (1000, 1000, 1000) and not os.getgroups(), "unexpected service groups")
status = dict(line.split(":\t", 1) for line in Path("/proc/self/status").read_text().splitlines() if ":\t" in line)
for field in ("CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"):
    require(int(status[field], 16) == 0, f"unexpected {field}")
require(status["NoNewPrivs"].strip() == "1", "expected no-new-privileges")
program = "/usr/local/bin/" + {"a":"spiralcore-mailbox-producer","b":"spiralcore-cast-recipient"}[role]
wrapper = "/usr/lib/mvm/wrappers/oci-entrypoint"
files = {
    "/etc/mvm/entrypoint": 0o644, "/etc/mvm/image-runtime.json": 0o644,
    "/etc/mvm/runtime.json": 0o444, "/etc/mvm/verb-trust.json": 0o444,
    "/etc/mvm/variant": 0o644, "/etc/mvm/name": 0o644,
    "/etc/passwd": 0o644, "/etc/group": 0o644, wrapper:0o555, program:0o555,
}
dirs = sorted({str(p) for f in files for p in Path(f).parents})
rows = []
contents = {}
for name in dirs + list(files):
    path = mount / name.lstrip("/")
    st = path.lstat()
    mode = stat.S_IMODE(st.st_mode)
    require(st.st_uid == 0 and st.st_gid == 0, f"unexpected owner {name}")
    if name in dirs:
        require(stat.S_ISDIR(st.st_mode), f"not a directory: {name}")
        require(os.access(path, os.X_OK, effective_ids=True), f"service cannot traverse {name}")
        require(mode == 0o755, f"unexpected directory mode {name}: {mode:04o}")
    else:
        require(stat.S_ISREG(st.st_mode) and mode == files[name], f"unexpected mode/type {name}")
        contents[name] = path.read_bytes()
        if name in (wrapper, program):
            require(contents[name].startswith(b"\x7fELF"), f"not an ELF: {name}")
            require(os.access(path, os.X_OK, effective_ids=True), f"service cannot execute {name}")
    rows.append({"path":name, "uid":st.st_uid, "gid":st.st_gid,"mode":f"{mode:04o}"})
require(contents["/etc/mvm/entrypoint"].decode().strip() == wrapper, "entrypoint marker mismatch")
require(json.loads(contents["/etc/mvm/image-runtime.json"])["argv"] == [program], "workload argv mismatch")
worker = json.loads(contents["/etc/mvm/runtime.json"])
require(worker["source_path"] == program and worker["concurrency"]["pool_size"] == 1, "worker config mismatch")
require(json.loads(contents["/etc/mvm/verb-trust.json"])["require_grant"] is True, "missing grant requirement")
require(hashlib.sha256(contents[program]).hexdigest() == program_digest, "workload executable differs")
require(hashlib.sha256(contents[wrapper]).hexdigest() == wrapper_digest, "entrypoint wrapper differs")
print(json.dumps(dict(role=role, uid=1000, gid=1000, supplementary_groups=[], effective_capabilities=0,
                      readonly=True, no_new_privileges=True, files_and_directories=rows), indent=2))
