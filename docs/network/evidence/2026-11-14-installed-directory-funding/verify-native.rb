require 'open3'
require 'json'
require 'digest'
root = '/Users/casey.allard/.codex/worktrees/mvm-journal-retirement/source'
source = '9e56be556720e427013cce4076aeecd0b561a065'
tree = '42f3dbe791234790e7653446b4e9453044f77367'
core = 'git+https://github.com/R4-Laboratories/SpiralCore?rev=29393e9b476daeef41f51485d2fb8829c4bbefde#29393e9b476daeef41f51485d2fb8829c4bbefde'
expected_locked = %w[spiralcore-directory-macos spiralcore-mvm-flowmux spiralcore-storage-windows spiralcore-transfer uor-r4-object-store]
results = ARGV.map do |zip|
  _, err, status = Open3.capture3('unzip', '-tqq', zip)
  abort "ZIP CRC #{zip}: #{err}" unless status.success?
  output, _, status = Open3.capture3('unzip', '-Z1', zip)
  abort 'ZIP listing' unless status.success?
  names = output.lines.map(&:chomp)
  abort 'unsafe/duplicate ZIP paths' unless names.uniq == names && names.all? { |n| !n.start_with?('/') && !n.split('/').include?('..') }
  rows = names.to_h { |n| data, _, ok = Open3.capture3('unzip', '-p', zip, n); abort n unless ok.success?; [n, data] }
  abort 'consumer commit/tree' unless rows.fetch('source-and-tree.txt').lines.map(&:chomp) == [source, tree]
  abort 'declared source' unless rows.fetch('declared-source.txt').strip == "git+https://github.com/Casey-allard/mvm?rev=#{source}##{source}"
  platform = rows.fetch('rustc.txt')[/^host: (.+)$/, 1]
  abort 'compiler host' unless %w[x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu aarch64-apple-darwin].include?(platform)
  native_names = %w[spiralcore-mvm-flowmux spiralcore-transfer uor-r4-object-store]
  native_names.unshift('spiralcore-directory-macos') if platform.end_with?('apple-darwin')
  packages = JSON.parse(rows.fetch('shared-core.json'))
  abort 'native package identity' unless packages.map { |p| p.fetch('name') }.sort == native_names && packages.all? { |p| p.fetch('source') == core }
  abort 'all locked core names' unless rows.fetch('shared-core-lock-names.txt').lines.map(&:chomp) == expected_locked
  abort 'all locked core pins' unless rows.fetch('shared-core-lock-sources.txt').lines.map(&:strip) == Array.new(5, "source = \"#{core}\"")
  abort 'executable drift' unless rows.fetch('executable-before.sha256') == rows.fetch('executable-after.sha256')
  manifest_names = rows.fetch('source.sha256').lines.map do |line|
    hash, name = line.chomp.split(/\s+/, 2)
    abort "native source #{name}" unless Digest::SHA256.file(File.join(root, name)).hexdigest == hash
    name
  end
  abort 'native source count' unless manifest_names.size == 4679 && manifest_names.uniq.size == 4679
  after_names = rows.fetch('source-after.txt').lines.map do |line|
    abort 'source recheck' unless line.end_with?(": OK\n")
    value = line.chomp.delete_suffix(': OK')
    value.start_with?("'") && value.end_with?("'") ? value[1...-1] : value
  end
  abort 'source after coverage' unless after_names == manifest_names
  controls = 8.times.map do |index|
    text = rows.fetch("control-#{index}.txt")
    abort "control #{index} result" unless text.match?(/^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured;/)
    selector = text[/^test (supervisor::\S+) \.\.\. ok$/, 1]
    abort 'selector' unless selector && rows.fetch('test-list.txt').include?("#{selector}: test\n")
    costs = %w[real user sys].to_h { |key| value = text[/^#{key} ([0-9.]+)$/, 1]; abort key unless value; [key, value.to_f] }
    {selector: selector, command_seconds: costs}
  end
  abort 'distinct controls' unless controls.map { |c| c[:selector] }.uniq.size == 8
  drop_ns = rows.fetch('control-6.txt')[/^STORE_DROP_ORDER called=true ledger_held=true returned_credit_visible=false original_owner_released=true retirement_wall_ns=([0-9]+)$/, 1]
  abort 'positive drop ordering' unless drop_ns
  abort 'executed controls' unless rows.fetch('counts.txt').include?("executed_controls=8\n")
  {artifact: File.basename(zip), sha256: Digest::SHA256.file(zip).hexdigest, member_hashes: rows.transform_values { |b| Digest::SHA256.hexdigest(b) }, compiler_host: platform, consumer_source_files: manifest_names.size, native_core_packages: native_names, host_nofile: rows.fetch('host-nofile.txt'), original_store_retirement_wall_ns: drop_ns.to_i, controls: controls, scope: 'native host-process fixtures; no booted MVM or WAN'}
end
abort 'three native hosts' unless results.map { |r| r[:compiler_host] }.sort == %w[aarch64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu]
puts JSON.pretty_generate(results)
