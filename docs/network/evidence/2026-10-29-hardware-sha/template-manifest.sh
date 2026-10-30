#!/bin/bash
set -euo pipefail
template_root=/Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment/templates
cd "$template_root"
rg --files --hidden --no-ignore | LC_ALL=C sort | while IFS= read -r path; do
  [[ -f "$path" && ! -L "$path" ]]
  shasum -a 256 "$path"
done
