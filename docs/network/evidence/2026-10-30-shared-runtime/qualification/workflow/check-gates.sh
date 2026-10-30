#!/usr/bin/env bash
set -euo pipefail
workflow=${1:?workflow path required}
[[ -f "$workflow" ]] || { printf '%s\n' 'FAIL: workflow source missing'; exit 1; }
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
ruby -ryaml - "$workflow" "$scratch" <<'RUBY'
workflow, scratch = ARGV
doc = YAML.safe_load(File.read(workflow))
trigger = doc['on'] || doc[true]
raise 'unexpected triggers' unless trigger.keys == ['push'] && trigger['push']['branches'] == ['codex/spiralcore-shared-runtime']
raise 'excess permissions' unless doc['permissions'] == {'contents'=>'read'}
raise 'wrong concurrency' unless doc.dig('concurrency','group') == 'spiralcore-shared-runtime-pilot' && doc.dig('concurrency','cancel-in-progress') == false
job = doc.fetch('jobs').fetch('pilot')
raise 'wrong budget' unless job['timeout-minutes'] == 20
gate = job.fetch('if')
%w[github.sha vars.SPIRALCORE_RUNTIME_SOURCE github.run_attempt].each { |s| raise "missing #{s}" unless gate.include?(s) }
raise 'missing attempt gate' unless gate.include?('github.run_attempt == 1')
raise 'missing exact source gate' unless gate.include?('github.sha == vars.SPIRALCORE_RUNTIME_SOURCE')
raise 'wrong branch gate' unless gate.include?("github.ref == 'refs/heads/codex/spiralcore-shared-runtime'")
steps = job.fetch('steps')
steps.each_with_index do |step,index|
  if step['uses']
    raise 'unpinned action' unless step['uses'].match?(/@[0-9a-f]{40}\z/)
  end
  if step['run']
    File.write(File.join(scratch,"step-#{index}.sh"),step['run'])
  end
end
raise 'missing frozen-vector gate' unless steps.any? { |s| s.fetch('run','').include?('native_canonical_checkpoint_and_trace_vectors_are_stable') }
raise 'missing locked source tests' unless steps.any? { |s| s.fetch('run','').include?('test --release --locked') && s['run'].include?('--all-targets') }
upload = steps.find { |s| s.fetch('uses','').start_with?('actions/upload-artifact@') }
raise 'missing unconditional retention' unless upload && upload['if']=='always()'
auth_steps = steps.select { |s| s.fetch('env',{}).values.any? { |v| v.to_s.include?('secrets.') } }
raise 'credential scope' unless auth_steps.length==1 && auth_steps[0].dig('env','SPIRALCORE_WAN_TOKEN')=='${{ secrets.SPIRALCORE_RUNTIME_TOKEN }}'
raise 'old configuration namespace' if File.read(workflow).match?(/(?:vars|secrets)\.SPIRALCORE_WAN_/)
all = steps.map { |s| s.fetch('run','') }.join("\n")
{'config'=>'CONFIG','auth'=>'AUTH'}.each do |name,marker|
  body = all[/# BEGIN #{marker} GATE\n(.*?)# END #{marker} GATE/m,1]
  raise "missing #{name} gate" unless body
  File.write(File.join(scratch,"#{name}.sh"),body)
end
puts 'PASS: workflow structure, source/attempt gates, secret isolation and retention'
RUBY
for script in "$scratch"/step-*.sh; do
  bash -n "$script"
  shellcheck --shell=bash --severity=warning "$script"
done
valid=(
  "GITHUB_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  "RUNTIME_SOURCE=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  "GITHUB_REF=refs/heads/codex/spiralcore-shared-runtime"
  "GITHUB_RUN_ATTEMPT=1"
  "RUNTIME_URL=https://test-runtime.trycloudflare.com/"
  "RUNTIME_RUN=shared-runtime-pilot-01"
  "RUNTIME_RECEIVER_SHA=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
)
env "${valid[@]}" bash "$scratch/config.sh"
count=0
for invalid in \
  'GITHUB_SHA=abcd' \
  'RUNTIME_SOURCE=cccccccccccccccccccccccccccccccccccccccc' \
  'GITHUB_REF=refs/heads/main' \
  'GITHUB_RUN_ATTEMPT=2' \
  'RUNTIME_URL=http://test-runtime.trycloudflare.com/' \
  'RUNTIME_URL=https://test-runtime.trycloudflare.com.attacker.example/' \
  'RUNTIME_URL=https://name@host.trycloudflare.com/' \
  'RUNTIME_URL=https://host.trycloudflare.com:443/' \
  'RUNTIME_URL=https://host.trycloudflare.com/path' \
  'RUNTIME_URL=https://host.trycloudflare.com/?token=hidden' \
  'RUNTIME_URL=https://-invalid.trycloudflare.com/' \
  'RUNTIME_RUN=../reuse' \
  'RUNTIME_RUN=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' \
  'RUNTIME_RECEIVER_SHA=not-a-sha256'; do
  if env "${valid[@]}" "$invalid" bash "$scratch/config.sh" > "$scratch/rejected.log" 2>&1; then
    printf '%s\n' 'FAIL: invalid source/origin/config accepted'; exit 1
  fi
  count=$((count+1))
done
env SPIRALCORE_WAN_TOKEN=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef bash "$scratch/auth.sh"
for invalid in '' short AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA; do
  if env SPIRALCORE_WAN_TOKEN="$invalid" bash "$scratch/auth.sh" > "$scratch/rejected.log" 2>&1; then
    printf '%s\n' 'FAIL: invalid capability accepted'; exit 1
  fi
  count=$((count+1))
done
printf 'PASS: valid configuration/capability and %s rejected configurations/capabilities\n' "$count"
printf '%s\n' 'PASS: all embedded shell blocks parse and pass ShellCheck warning gate'
