#!/bin/bash
set -euo pipefail
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$probe_root"
[[ ! -e identity-qualification.json && ! -e identity-execution.jsonl ]]
bash verify-inputs.sh
for configuration in software hardware; do
  shasum -a 256 -c "build-$configuration/executables.sha256"
  jq -nc --arg configuration "$configuration" '{event:"start",configuration:$configuration}' >> identity-execution.jsonl
  set +e
  "build-$configuration/identity" "$probe_root/identity-$configuration" > "identity-$configuration.stdout" 2> "identity-$configuration.stderr"
  code=$?
  set -e
  jq -nc --arg configuration "$configuration" --arg code "$code" '{event:"end",configuration:$configuration,exit_code:($code|tonumber)}' >> identity-execution.jsonl
  [[ "$code" == 0 ]]
  jq -se 'length == 36 and all(.exact_rebuild == true) and ([.[]|select(.case=="receiver-base")|.object_id] == ["c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0"]) and ([.[]|select(.case=="receiver-target")|.object_id] == ["227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf"])' "identity-$configuration/identities.jsonl" > /dev/null
  (cd "identity-$configuration"; shasum -a 256 identities.jsonl case-*.nodes) > "identity-$configuration.sha256"
  # Negative qualification: an existing output directory must remain byte-identical.
  set +e
  "build-$configuration/identity" "$probe_root/identity-$configuration" > "identity-$configuration-reuse.stdout" 2> "identity-$configuration-reuse.stderr"
  reuse_code=$?
  "build-$configuration/identity" "identity-relative-$configuration" > "identity-$configuration-relative.stdout" 2> "identity-$configuration-relative.stderr"
  relative_code=$?
  set -e
  [[ "$reuse_code" != 0 && "$relative_code" != 0 && ! -e "identity-relative-$configuration" ]]
  (cd "identity-$configuration"; shasum -a 256 -c "../identity-$configuration.sha256") > "identity-$configuration-unchanged.txt"
  jq -nc --arg configuration "$configuration" --arg reuse "$reuse_code" --arg relative "$relative_code" '{event:"negative_checks",configuration:$configuration,reuse_exit:($reuse|tonumber),relative_exit:($relative|tonumber),original_files_unchanged:true,relative_path_not_created:true}' >> identity-execution.jsonl
done
cmp identity-software/identities.jsonl identity-hardware/identities.jsonl
for file in identity-software/case-*.nodes; do cmp "$file" "identity-hardware/$(basename "$file")"; done
cmp identity-software.sha256 identity-hardware.sha256
jq -nc '{schema:"spiralcore.sha-identity-qualification.v1",status:"passed",fixtures:36,configurations:2,identity_outputs_equal:true,complete_node_files_equal:true,manifest_encodings_equal:true,object_and_manifest_ids_equal:true,independent_receiver_fixture_ids_match:true,negative_existing_output_rejected:true,negative_existing_files_unchanged:true,negative_relative_path_rejected:true,measured_probe_rust_unchanged:true,timing:false}' > identity-qualification.json
