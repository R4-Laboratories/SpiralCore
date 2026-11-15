require 'json'
base='0402b182bfc10b7555298d05cc91e6bf216c7982'
head='44e895503072984a05fa61c00e832aeedcc70389'
paths=IO.popen(['git','diff','--diff-filter=M',base,head,'--name-only'], &:read).lines.map(&:strip).select{|p|p.end_with?('Cargo.lock')}
helper="[[package]]\nname = \"spiralcore-directory-macos\"\nversion = \"0.1.0\"\ndependencies = [\n \"libc\",\n]\n\n"
rows=paths.map do |path|
 before=IO.popen(['git','show',"#{base}:#{path}"], &:read)
 after=IO.popen(['git','show',"#{head}:#{path}"], &:read)
 raise "Unexpected helper count: #{path}" unless after.scan(helper).size==1
 restored=after.sub(helper,'')
 restored.sub!(/(name = "spiralcore-transfer"\n.*?dependencies = \[\n)(.*?)(\n\])/m) do
  prefix,deps,suffix=Regexp.last_match.captures
  raise "Unexpected helper edge count: #{path}" unless deps.scan(" \"spiralcore-directory-macos\",\n").size==1
  prefix+deps.sub(" \"spiralcore-directory-macos\",\n",'')+suffix
 end
 raise "Unexpected full lockfile change: #{path}" unless restored==before
 {path:path,existing_packages_unchanged:true,validation:'exact full-text equality after reversing only helper package and transfer edge; preserves duplicate package versions',added_package:'spiralcore-directory-macos',added_edge:'spiralcore-transfer -> spiralcore-directory-macos'}
end
raise 'Expected ten existing locks' unless rows.size==10
File.write('/tmp/sc-258-directory/lock-validation.json',JSON.pretty_generate({base:base,source:head,locks:rows})+"\n")
puts 'All ten complete lockfiles validated'
