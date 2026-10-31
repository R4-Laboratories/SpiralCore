#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/persistent-session-20261031/coverage-diagnostic
evidence=docs/network/evidence/2026-10-31-persistent-session/qualification
mkdir "$root"
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/local/shared-state-20261030/coverage-target
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-C instrument-coverage"
export LLVM_PROFILE_FILE="$root/diag-%p-%m.profraw"
/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --target aarch64-apple-darwin --features hardware-sha256 --test wan --test wan_persistent --test wan_diagnostics_cli --message-format=json > "$root/cargo-output.jsonl" 2> "$evidence/diagnostic-coverage-stderr.txt"
python3 - <<'INNER'
from pathlib import Path
import json, subprocess, hashlib
root=Path('/Users/casey.allard/spiralcore/local/persistent-session-20261031/coverage-diagnostic')
evidence=Path('docs/network/evidence/2026-10-31-persistent-session/qualification')
objects=set(); lines=[]
for line in (root/'cargo-output.jsonl').read_text().splitlines():
 try: row=json.loads(line)
 except ValueError:
  lines.append(line);continue
 if row.get('reason')=='compiler-artifact' and row.get('executable'):
  objects.add(row['executable'])
assert objects
(evidence/'diagnostic-coverage-tests.txt').write_text('\n'.join(lines)+'\n')
profiles=sorted(root.glob('*.profraw'));assert profiles
llvm=Path('/Users/casey.allard/.rustup/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin')
subprocess.run([str(llvm/'llvm-profdata'),'merge','-sparse',*[str(p) for p in profiles],'-o',str(root/'current.profdata')],check=True)
args=[str(llvm/'llvm-cov'),'export','-summary-only','-instr-profile='+str(root/'current.profdata')]
for p in sorted(objects): args+=['-object',p]
with (evidence/'diagnostic-coverage.json').open('wb') as out: subprocess.run(args,stdout=out,check=True)
for name, paths in [('objects',[Path(p) for p in sorted(objects)]),('profiles',profiles),('sources',[Path('crates/spiralcore-transfer/src/wan.rs'),Path('crates/spiralcore-transfer/src/bin/spiralcore-wan.rs')])]:
 (evidence/f'diagnostic-coverage-{name}.sha256').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p)+'\n' for p in paths))
data=json.loads((evidence/'diagnostic-coverage.json').read_text())
for file in data['data'][0]['files']:
 if file['filename'].endswith(('/src/wan.rs','/src/bin/spiralcore-wan.rs')): print(file['filename'],file['summary']['lines'])
INNER
