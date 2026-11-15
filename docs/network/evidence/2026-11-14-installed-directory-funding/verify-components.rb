require 'open3'
require 'json'
require 'digest'
lock = Digest::SHA256.file('/Users/casey.allard/.codex/worktrees/spiralcore-mvm-resume/spiralcore/crates/spiralcore-mvm-flowmux/Cargo.lock').hexdigest
windows_lock = Digest::SHA256.hexdigest(File.binread("/Users/casey.allard/.codex/worktrees/spiralcore-mvm-resume/spiralcore/crates/spiralcore-mvm-flowmux/Cargo.lock").gsub("\n", "\r\n"))
results = ARGV.map do |zip|
  _, err, ok = Open3.capture3('unzip', '-tqq', zip); abort err unless ok.success?
  names, _, ok = Open3.capture3('unzip', '-Z1', zip); abort 'listing' unless ok.success?
  names = names.lines.map(&:chomp)
  abort 'paths' unless names.uniq == names && names.all? { |n| !n.start_with?('/') && !n.split('/').include?('..') }
  rows = names.to_h { |name| data, _, ok = Open3.capture3('unzip', '-p', zip, name); abort name unless ok.success?; [name, data] }
  checked = rows.fetch('SHA256SUMS').lines.map do |line|
    hash, name = line.strip.split(/\s+/, 2);name = name.delete_prefix('*')
    abort "embedded checksum #{name}" unless Digest::SHA256.hexdigest(rows.fetch(name)) == hash.downcase
    name
  end
  abort 'checksum coverage' unless checked.uniq == checked && checked.sort == (names - ['SHA256SUMS']).sort
  identity = JSON.parse(rows.fetch('identity.json'))
  abort 'component identity' unless identity.fetch('source') == 'f59f2f8a713bdda583eb18a6a71f126966911cca' && identity.fetch('run') == '43675832377' && identity.fetch('lock_sha256').downcase == (identity.fetch('os') == 'Windows' ? windows_lock : lock)
  counts = JSON.parse(rows.fetch('counts.json'))
  abort '87 component tests' unless counts.values_at('listed','passed','failed','ignored') == [87,87,0,0]
  bulk = rows.fetch('tests.txt').lines.find { |line| line.start_with?('ORDERED_BULK ') }
  abort 'ordered bulk record' unless bulk
  fields = %w[blocks_each_direction exact_bytes_each_direction actual_tls_a_sent actual_tls_a_received peer_sent peer_received setup_wall_us lifecycle_wall_us].to_h { |key| value = bulk[/\b#{key}=([0-9]+)/,1];abort key unless value;[key,value.to_i] }
  abort 'exact bulk bytes' unless fields.fetch('blocks_each_direction') == 4096 && fields.fetch('exact_bytes_each_direction') == 67108864
  abort 'wire direction agreement' unless fields.fetch('actual_tls_a_sent') == fields.fetch('peer_received') && fields.fetch('actual_tls_a_received') == fields.fetch('peer_sent')
  cpu = bulk[/lifecycle_process_cpu_us=Some\(([0-9]+)\)/,1];abort 'actual CPU' unless cpu
  {artifact: File.basename(zip), sha256: Digest::SHA256.file(zip).hexdigest, identity: identity, members: rows.size, embedded_checksums: checked.size, member_sha256: rows.transform_values { |b| Digest::SHA256.hexdigest(b) }, flowmux: counts, ordered_bulk: fields.merge('lifecycle_process_cpu_us' => cpu.to_i), raw_bulk_record: bulk.strip, scope: 'native TCP/TLS/shared-object component; no MVM backend or WAN'}
end
abort 'seven native components' unless results.size == 7
puts JSON.pretty_generate(results)
