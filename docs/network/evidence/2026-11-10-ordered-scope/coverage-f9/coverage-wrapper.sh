#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  /var/tmp/spiralcore-f9-timing-coverage-20261110/source/crates/spiralcore-mvm-flowmux|/var/tmp/spiralcore-f9-timing-coverage-20261110/source/crates/spiralcore-transfer) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
