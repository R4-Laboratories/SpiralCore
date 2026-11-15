require 'rubygems/package'
require 'zlib'
require 'digest'
require 'json'
archive = ARGV.fetch(0)
rows = {}
Gem::Package::TarReader.new(Zlib::GzipReader.open(archive)) do |tar|
  tar.each do |entry|
    name = entry.full_name
    abort "unsafe/duplicate member #{name}" unless entry.file? && name == File.basename(name) && !rows.key?(name)
    rows[name] = entry.read || String.new
  end
end
manifest = rows.fetch('mvm-directory-raw-members.sha256').lines
manifest.each do |line|
  hash, name = line.chomp.split('  ', 2)
  abort "checksum #{name}" unless Digest::SHA256.hexdigest(rows.fetch(name)) == hash
end
abort 'unlisted archive member' unless rows.size == manifest.size + 1
before = rows.fetch('mvm-directory-source.sha256').lines
root = '/Users/casey.allard/.codex/worktrees/mvm-journal-retirement/source'
before.each do |line|
  hash, name = line.chomp.split('  ', 2)
  abort "consumer source #{name}" unless Digest::SHA256.file(File.join(root, name)).hexdigest == hash
end
%w[mvm-directory-source-before.txt mvm-directory-workspace-final-source-before.txt mvm-directory-final-source-after.txt mvm-directory-post-source-before.txt mvm-directory-post-source-after.txt].each do |name|
  lines = rows.fetch(name).lines
  abort name unless lines.length == before.length && lines.all? { |line| line.end_with?(": OK\n") }
end
%w[mvm-directory-registered-isolated mvm-directory-fd-baseline-control mvm-directory-fd-candidate-control].each do |prefix|
  abort prefix unless rows.fetch("#{prefix}-before.sha256") == rows.fetch("#{prefix}-after.sha256")
end
old = 'adcc7197aa277277be5d074383c6267476163206#adcc7197'
new = '29393e9b476daeef41f51485d2fb8829c4bbefde#29393e9b'
baseline_manifest = File.join(File.dirname(ARGV[1] || archive), "baseline-611-source.sha256")
if File.file?(baseline_manifest)
  manifest_text = File.read(baseline_manifest)
  abort "baseline manifest digest" unless Digest::SHA256.hexdigest(manifest_text) == "16d55ab597c2e17eb2907147c1945255479f1d047dcaf0a2c043b9564f1a763a"
  expected_names = manifest_text.lines.map { |line| line.chomp.split("  ", 2).last }
  %w[mvm-directory-fd-baseline-source-before.txt mvm-directory-fd-baseline-source-after.txt].each do |name|
    receipt_names = rows.fetch(name).lines.map { |line| value = line.chomp.delete_suffix(": OK"); value.start_with?("\x27") && value.end_with?("\x27") ? value[1...-1] : value }
    abort "baseline receipt binding #{name}" unless receipt_names == expected_names
  end
else
  abort "required adjacent baseline-611-source.sha256 is missing"
end
graphs = %w[x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc aarch64-pc-windows-msvc mvmctl mvm-builder-guest].map do |target|
  a = rows.fetch("mvm-directory-matched-#{target}-baseline.txt").gsub(old, new)
  b = rows.fetch("mvm-directory-matched-#{target}-candidate.txt")
  if target.end_with?('apple-darwin')
    lines = b.lines
    index = lines.index { |line| line.include?('spiralcore-directory-macos feature "default"') }
    abort "helper graph #{target}" unless index && lines[index+1].include?('spiralcore-directory-macos v0.1.0') && lines[index+2].include?('libc feature "default" (*)')
    lines.slice!(index, 3)
    b = lines.join
  end
  abort "unexpected graph change #{target}" unless a == b
  {target: target, result: 'PASS', scope: 'Cargo graph resolution on Linux; native execution is separate'}
end
abort 'VMM dependency drift' unless rows.fetch('mvm-directory-vmm-closure-baseline.txt') == rows.fetch('mvm-directory-vmm-closure-candidate.txt')
control_rows = rows.fetch('mvm-directory-fd-controls.txt').lines.map(&:split)
abort '40 FD controls' unless control_rows.size == 40
fd = %w[baseline candidate].flat_map { |v| %w[1024 4096].map { |limit| group = control_rows.select { |r| r[0] == v && r[1] == limit }; abort 'FD control count' unless group.size == 10; {variant: v, soft_nofile: limit.to_i, pass: group.count { |r| r[3] == '0' }, fail: group.count { |r| r[3] != '0' }} } }
result = {archive_sha256: Digest::SHA256.file(archive).hexdigest, raw_members: rows.size, embedded_checksums: manifest.size, exact_consumer_source_files: before.size, final_workspace_exit_zero: rows.fetch('mvm-directory-workspace-final.log').include?("Exit status: 0"), source_and_executable_rechecks: 'PASS', graph_pairs: graphs, fd_controls: fd, vmm_closure: 'byte-identical', scope: 'Native Wick Linux host-process fixtures; no MVM/WAN/capacity acceptance'}
abort 'workspace not successful' unless result[:final_workspace_exit_zero]
puts JSON.pretty_generate(result)
