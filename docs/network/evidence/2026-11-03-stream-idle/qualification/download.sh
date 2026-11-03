#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
gh api repos/R4-Laboratories/SpiralCore/actions/runs/41638793063 > portable/run.json
gh api repos/R4-Laboratories/SpiralCore/actions/runs/41638793063/artifacts > portable/artifacts.json
gh api repos/R4-Laboratories/SpiralCore/actions/runs/41638793063/jobs > portable/jobs.json
gh api repos/R4-Laboratories/SpiralCore/actions/runs/41638793269 > linux-run.json
gh api repos/R4-Laboratories/SpiralCore/actions/runs/41638793269/jobs > linux-jobs.json
gh run view 41638793269 --repo R4-Laboratories/SpiralCore --log > linux-run.log
jq -r '.artifacts[]|[.id,.name]|@tsv' portable/artifacts.json | while IFS=$'\t' read -r id name; do
  [[ "$id" =~ ^[0-9]+$ && "$name" =~ ^mvm-mailbox-portable-[a-z0-9-]+-1$ ]]
  gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "portable/zips/$name.zip"
done
