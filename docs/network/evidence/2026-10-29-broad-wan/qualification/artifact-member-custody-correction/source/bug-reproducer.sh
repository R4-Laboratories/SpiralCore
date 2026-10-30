#!/bin/bash
set -euo pipefail
# Minimal reproducer of the historical defect, not the full historical command.
# Missing input redirection causes zero visits with EOF stdin. No count gate follows.
members=$1
output=$2
while IFS= read -r member; do
  printf '%s\n' "$member"
done > "$output"
printf 'claimed_members=%s actual_output_lines=%s\n' "$(wc -l < "$members" | tr -d ' ')" "$(wc -l < "$output" | tr -d ' ')"
