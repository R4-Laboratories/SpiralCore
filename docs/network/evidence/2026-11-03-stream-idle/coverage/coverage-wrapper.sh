#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */crates/spiralcore-mvm-adapter|*/crates/spiralcore-mvm-live)
    exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
