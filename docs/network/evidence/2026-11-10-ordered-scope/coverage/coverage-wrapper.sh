#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */spiralcore-ordered-coverage-f26b7803-20261110/source/crates/spiralcore-mvm-flowmux|*/spiralcore-ordered-coverage-f26b7803-20261110/source/crates/spiralcore-transfer) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
