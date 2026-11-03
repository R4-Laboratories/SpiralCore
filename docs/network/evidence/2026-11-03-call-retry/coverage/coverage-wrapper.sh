#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */crates/spiralcore-mvm-adapter) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
