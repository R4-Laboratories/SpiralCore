#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */crates/spiralcore-mvm-adapter|*/crates/spiralcore-mvm-live|*/crates/spiralcore-transfer|*/crates/spiralcore-gateway-core|*/crates/uor-r4-object-store)
    exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
