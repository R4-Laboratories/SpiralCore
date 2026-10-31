#!/usr/bin/env bash
set -euo pipefail
compiler=$1
shift
case " $* " in
  *' --crate-name spiralcore_mvm_live '*|*' --crate-name spiralcore_mvm_adapter '*|*' --crate-name adapter_live '*|*' --crate-name campaign '*|*' --crate-name host_pack '*) exec "$compiler" -C instrument-coverage "$@" ;;
  *) exec "$compiler" "$@" ;;
esac
