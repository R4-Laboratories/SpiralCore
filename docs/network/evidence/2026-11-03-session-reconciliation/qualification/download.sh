#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
case "${1:?portable or linux}" in
 portable)
  [[ ! -e portable/run.json ]]
  gh api repos/R4-Laboratories/SpiralCore/actions/runs/41677859307 > portable/run.json
  jq -e '.head_sha=="b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea" and .status=="completed" and .conclusion=="success" and .run_attempt==1' portable/run.json >/dev/null
  gh api repos/R4-Laboratories/SpiralCore/actions/runs/41677859307/artifacts > portable/artifacts.json
  gh api repos/R4-Laboratories/SpiralCore/actions/runs/41677859307/jobs > portable/jobs.json
  jq -r '.artifacts[]|[.id,.name]|@tsv' portable/artifacts.json | while IFS=$'\t' read -r id name; do
   [[ "$id" =~ ^[0-9]+$ && "$name" =~ ^mvm-mailbox-portable-[a-z0-9-]+-1$ ]]
   [[ ! -e "portable/archives/$name.zip" ]]
   gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "portable/archives/$name.zip"
  done
  ;;
 linux)
  [[ ! -e linux/run.json ]]
  gh api repos/R4-Laboratories/SpiralCore/actions/runs/41677859354 > linux/run.json
  jq -e '.head_sha=="b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea" and .status=="completed" and .conclusion=="success" and .run_attempt==1' linux/run.json >/dev/null
  gh api repos/R4-Laboratories/SpiralCore/actions/runs/41677859354/jobs > linux/jobs.json
  gh run view 41677859354 --repo R4-Laboratories/SpiralCore --log > linux/run.log
  ;;
 *) exit 2 ;;
esac
