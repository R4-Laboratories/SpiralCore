#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case "${CARGO_MANIFEST_DIR:-}" in
  */installed-supervisor-harness) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
