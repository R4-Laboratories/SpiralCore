#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
case "${1:?portable or linux}" in
  portable)
    gh api repos/R4-Laboratories/SpiralCore/actions/runs/41649886300 > portable/run.json
    gh api repos/R4-Laboratories/SpiralCore/actions/runs/41649886300/artifacts > portable/artifacts.json
    gh api repos/R4-Laboratories/SpiralCore/actions/runs/41649886300/jobs > portable/jobs.json
    jq -r '.artifacts[]|[.id,.name]|@tsv' portable/artifacts.json | while IFS=$'\t' read -r id name; do
      [[ "$id" =~ ^[0-9]+$ && "$name" =~ ^mvm-mailbox-portable-[a-z0-9-]+-1$ ]]
      [[ ! -e "portable/zips/$name.zip" ]]
      gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "portable/zips/$name.zip"
    done
    ;;
  linux)
    gh api repos/R4-Laboratories/SpiralCore/actions/runs/41649886275 > linux-run.json
    gh api repos/R4-Laboratories/SpiralCore/actions/runs/41649886275/jobs > linux-jobs.json
    gh run view 41649886275 --repo R4-Laboratories/SpiralCore --log > linux-run.log
    ;;
  *) exit 2 ;;
esac
