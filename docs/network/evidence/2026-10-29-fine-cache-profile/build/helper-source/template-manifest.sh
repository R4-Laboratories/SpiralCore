#!/bin/bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$root/experiment"
for layout in empty compact64 compact768 spread768; do
  rg --files --hidden --no-ignore "$layout/templates"
done | LC_ALL=C sort | while IFS= read -r file; do
  [[ -f "$file" && ! -L "$file" ]]
  shasum -a 256 "$file"
done
