require 'json'
base='af0f0765ef5d8663d1de79a7522bfb653b1c5122'
paths=IO.popen(['git','diff','--name-only',base],&:read).lines.map(&:strip).select{|p|p.end_with?('Cargo.lock')}
parse=->(text) do
 chunks=text.split(/(?=\[\[package\]\])/).drop(1).map{|b|b.split(/(?=\[\[patch\.unused\]\])/).first.strip}
 chunks.map! do |block|
  block.sub(/dependencies = \[\n(.*?)\n\]/m) do
   deps=Regexp.last_match(1).lines.sort.join.chomp
   "dependencies = [\n#{deps}\n]"
  end
 end
 entries=chunks.map{|block| [[block[/^name = "([^"]+)"/,1],block[/^version = "([^"]+)"/,1],block[/^source = "([^"]+)"/,1]],block]}
 raise 'Duplicate package identity' unless entries.map(&:first).uniq.size==entries.size
 entries.to_h
end
rows=paths.map do |path|
 old=IO.popen(['git','show',"#{base}:#{path}"],&:read).gsub('611a3c76bf6f064760e5adef4fa7befbeb4eb650','9e56be556720e427013cce4076aeecd0b561a065').gsub('4ea98b9e27116a163c9767f5a351bf177eff3b6b','29393e9b476daeef41f51485d2fb8829c4bbefde')
 before=parse.call(old);after=parse.call(File.read(path))
 additions=after.keys-before.keys;removals=before.keys-after.keys
 raise "Unexpected removals #{path}" unless removals.empty?
 raise "Unexpected additions #{path}" unless additions.all?{|key|key[0]=='spiralcore-directory-macos' && key[1]=='0.1.0'}
 changed=[]
 before.each do |key,block|
  next if block==after[key]
  changed << key[0]
  raise "Unexpected package change #{path}:#{key.inspect}" unless key[0]=='spiralcore-transfer' && after[key].sub(" \"spiralcore-directory-macos\",\n",'')==block
 end
 {path:path,existing_package_versions_and_dependencies_preserved:true,normalized_sources:['MVM611 -> 9e56','installed tool SC4ea98 -> 29393'],additional_packages:additions,additional_edges:changed.map{|n|"#{n} -> spiralcore-directory-macos"},package_order_ignored:true,dependency_order_ignored:true}
end
File.write('/tmp/sc-258-mvm-directory/sdk-lock-validation.json',JSON.pretty_generate(rows)+"\n")
puts "Validated #{rows.size} complete SDK package maps"
