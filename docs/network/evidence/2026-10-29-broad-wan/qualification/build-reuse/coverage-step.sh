set -uo pipefail
printf '%s\n' release_identity_before > wan-results/coverage-stage.txt
if ! sha256sum -c wan-results/binary-sha256.txt > wan-results/release-before-coverage.txt 2>&1; then
  jq -n '{schema:"spiralcore.source-coverage.v1",stage:"release_identity_before",exit:1,release_unchanged:false}' > wan-results/coverage-status.json
  exit 1
fi
set +e
(
  set -euo pipefail
  printf '%s\n' install_tools > wan-results/coverage-stage.txt
  rustup component add llvm-tools-preview --toolchain 1.97.1
  CARGO_TARGET_DIR="$RUNNER_TEMP/coverage-install-target" cargo +1.97.1 install cargo-llvm-cov --version 0.9.1 --locked --root "$RUNNER_TEMP/coverage-tools"
  export PATH="$RUNNER_TEMP/coverage-tools/bin:$PATH"
  export CARGO_TARGET_DIR="$RUNNER_TEMP/spiralcore-coverage"
  export CARGO_LLVM_COV_TARGET_DIR="$CARGO_TARGET_DIR"
  # Instrumentation variables are confined to this subshell and fresh target.
  coverage_env=$(cargo +1.97.1 llvm-cov show-env --sh --manifest-path crates/spiralcore-transfer/Cargo.toml)
  eval "$coverage_env"
  printf '%s\n' all_target_tests_including_ignored > wan-results/coverage-stage.txt
  timeout 300s cargo +1.97.1 test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- --include-ignored
  printf '%s\n' instrumented_binaries > wan-results/coverage-stage.txt
  timeout 300s cargo +1.97.1 build --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --bins
  printf '%s\n' instrumented_capture_qualification > wan-results/coverage-stage.txt
  timeout 180s "$CARGO_TARGET_DIR/debug/spiralcore-capture-control" --qualify "$GITHUB_WORKSPACE/wan-results/instrumented-capture-qualification"
  printf '%s\n' coverage_report > wan-results/coverage-stage.txt
  cargo +1.97.1 llvm-cov report --manifest-path crates/spiralcore-transfer/Cargo.toml --json --summary-only --output-path "$GITHUB_WORKSPACE/wan-results/coverage.json"
  printf '%s\n' complete > wan-results/coverage-stage.txt
)
result=$?
# Check the release binaries even when instrumentation or qualification failed.
unchanged=false
if sha256sum -c wan-results/binary-sha256.txt > wan-results/release-after-coverage.txt 2>&1; then
  unchanged=true
else
  result=1
fi
jq -n --arg stage "$(cat wan-results/coverage-stage.txt)" --argjson result "$result" --argjson unchanged "$unchanged" \
  '{schema:"spiralcore.source-coverage.v1",stage:$stage,exit:$result,release_unchanged:$unchanged,coverage_tool:"cargo-llvm-cov 0.9.1",coverage_gate:"report_only",scope:"separate instrumented target; ordinary release capture qualification remains separate"}' > wan-results/coverage-status.json || exit 1
exit "$result"
