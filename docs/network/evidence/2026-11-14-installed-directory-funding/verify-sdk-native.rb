require 'open3'
require 'digest'
require 'json'
root = '/Users/casey.allard/.codex/worktrees/spiralcore-mvm-resume/spiralcore'
results = ARGV.map do |zip|
  _, err, ok = Open3.capture3('unzip', '-tqq', zip)
  abort err unless ok.success?
  names, _, ok = Open3.capture3('unzip', '-Z1', zip)
  abort 'ZIP listing' unless ok.success?
  names = names.lines.map(&:chomp)
  abort 'unsafe paths' unless names.uniq == names && names.all? { |n| !n.start_with?('/') && !n.split('/').include?('..') }
  rows = names.to_h { |name| data, _, ok = Open3.capture3('unzip', '-p', zip, name); abort name unless ok.success?; [name, data] }
  abort 'SDK candidate' unless rows.fetch('source-commit.txt').strip == 'a0bb0dba681a2c72aa59ea2d194f9fc2ca2694a4'
  manifest_names = rows.fetch('source.sha256').lines.map do |line|
    hash, name = line.chomp.split(/\s+/, 2)
    abort name unless Digest::SHA256.file(File.join(root, name)).hexdigest == hash
    name
  end
  rows.select { |name, _| name.match?(/^source-after(?:-collector)?\.txt$/) }.each do |name, text|
    abort name unless text.lines.all? { |l| l.end_with?(": OK\n") } && text.lines.map { |l| l.chomp.delete_suffix(': OK') } == manifest_names
  end
  expected = File.basename(zip).start_with?('installed-parent') ? [38,3,11] : [8]
  expected.each { |count| abort "test footer #{count}" unless rows.fetch('tests.txt').match?(/^test result: ok\. #{count} passed; 0 failed; 0 ignored; 0 measured;/) }
  platform = rows.fetch('rustc.txt')[/^host: (.+)$/, 1]
  if File.basename(zip).start_with?('canonical-endpoint') && platform.end_with?('linux-gnu')
    abort 'collector4' unless rows.fetch('collector-tests.txt').match?(/^test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured;/)
    abort 'bounded main stack' unless rows.fetch('stack-configured-kib.txt').strip == '8192'
  end
  {artifact: File.basename(zip), sha256: Digest::SHA256.file(zip).hexdigest, member_hashes: rows.transform_values { |b| Digest::SHA256.hexdigest(b) }, compiler_host: platform, source_files: manifest_names.size, expected_test_footers: expected, scope: 'native SDK host-process controls; no booted MVM or WAN'}
end
abort 'six SDK artifacts' unless results.size == 6
puts JSON.pretty_generate(results)
