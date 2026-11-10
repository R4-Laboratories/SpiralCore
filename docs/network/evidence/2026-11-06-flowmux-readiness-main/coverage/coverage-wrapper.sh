#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */spiralcore-pr104-coverage-8535b10d-20261110/source/crates/spiralcore-mvm-flowmux) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
