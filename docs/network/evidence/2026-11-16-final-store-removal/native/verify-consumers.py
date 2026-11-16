from pathlib import Path
import hashlib,json,re,sys,zipfile
root=Path(__file__).resolve().parent
frozen={line.split('  ',1)[1].removeprefix('./'):line.split('  ',1)[0] for line in (root/'frozen-mvm-inputs.sha256').read_text().splitlines()}
assert len(frozen)==4757
prefix='supervisor::endpoint::flowmux::spiralcore_payload::resource_pool_tests::'
expected=[
'supervisor::endpoint::flowmux::owned_relays::custody_tests::close_between_core_creation_and_attach_retains_original_reader_until_join',
'supervisor::endpoint::flowmux::owned_relays::custody_tests::poison_between_core_creation_and_attach_retains_original_reader_without_drain',
prefix+'replaced_installed_store_quarantines_original_locked_owner_after_endpoint_drop',
prefix+'partial_installed_store_retains_original_parent_name_and_debit',
prefix+'installed_store_workspace_is_charged_before_visits_are_available',
prefix+'store_cleanup_releases_only_exact_owned_identity_and_retains_failed_charge',
prefix+'installed_store_destruction_precedes_returned_credit',
prefix+'poisoned_store_ledger_retains_original_buffers_lock_and_debit',
'installed_endpoint::controller::network::worker_tests::serial_worker_quote_includes_all_selected_concurrent_owners',
'supervisor::endpoint::flowmux::credit_tests::returned_upload_credit_keeps_the_session_validator_current',
prefix+'owner_mode_tests::broad_mode_capture_refuses_and_retains_original_store_and_debit',
prefix+'owner_mode_tests::broad_mode_retirement_refuses_and_retains_original_lease_lock_and_debit',
prefix+'owner_mode_tests::foreign_uid_capture_refuses_and_retains_original_store_and_debit',
prefix+'owner_mode_tests::foreign_uid_retirement_refuses_and_retains_original_lease_lock_and_debit',
prefix+'retirement_failure_tests::final_rmdir_refusal_retains_unlinked_owner_lease_and_original_debit',
]
records=[]
for artifact in json.loads((root/'consumer-catalog.json').read_text()):
 path=root/(str(artifact['id'])+'-'+artifact['name']+'.zip');raw=path.read_bytes();assert len(raw)==artifact['bytes'] and hashlib.sha256(raw).hexdigest()==artifact['sha256']
 assert artifact['head']=='35efbb610e291c985fc16569f8ec8b6e3030bd98'
 target=artifact['name'].removeprefix('native-store-consumer-').removesuffix('-1')
 with zipfile.ZipFile(path) as z:
  assert z.testzip() is None
  def text(name):return z.read(name).decode()
  assert text('source-and-tree.txt').splitlines()==['cc2ed208a7b1adde7fa1f075bbfff2428f911e18','52e3463526c94a5941a16fa187158fd630c258bd']
  hashes={line.split('  ',1)[1]:line.split('  ',1)[0] for line in text('source.sha256').splitlines()};assert hashes==frozen
  after=text('source-after.txt').splitlines();assert len(after)==4757 and all(line.endswith(': OK') for line in after);assert {line[:-4] for line in after}==set(frozen)
  before=text('executable-before.sha256');assert before==text('executable-after.sha256')
  checksum,executable=before.strip().split('  ',1);assert re.fullmatch('[0-9a-f]{64}',checksum)
  build=[json.loads(line) for line in text('build.jsonl').splitlines()]
  selected=[r for r in build if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='mvm_hostd' and r['target']['kind']==['lib'] and r['profile']['test'] and r.get('executable')]
  assert len(selected)==1 and selected[0]['executable']==executable and selected[0]['fresh'] is False
  assert 'host: '+target in text('rustc.txt').splitlines()
  assert text('stack-configured-kib.txt').strip()=='8192'
  assert 'executed_controls=15' in text('counts.txt') and 'explicit_root_uid_controls=2' in text('counts.txt')
  observed=[]
  for i,selector in enumerate(expected):
   log=text(f'control-{i}.txt');assert selector+': test' in text('test-list.txt').splitlines()
   assert 'test '+selector+' ...' in log
   summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log,re.M);assert summaries==[('ok','1','0','0')],(target,i,summaries)
   clocks={key:value for key,value in re.findall(r'^(real|user|sys)\s+([0-9.]+)$',log,re.M)};assert set(clocks)=={'real','user','sys'}
   if i>=10:assert re.search(r'STORE_FIXTURE_OBJECT exact_bytes=49 serialized_in_memory_request_bytes=268 serialized_in_memory_reply_bytes=218 verified_wall_ns=\d+ network_bytes=0',log)
   observed.append({'index':i,'selector':selector,'result':'1 passed/0 failed/0 ignored','time_p_seconds':clocks})
  assert re.search(r'^STORE_DROP_ORDER called=true ledger_held=true returned_credit_visible=false original_owner_released=true retirement_wall_ns=\d+$',text('control-6.txt'),re.M)
  assert 'explicit_unprivileged_late_cleanup_controls=1' in text('counts.txt')
  assert re.search(r'^FINAL_RMDIR_REFUSAL owner_links=0 original_lock_retained=true original_child_retained=true quarantined=1 active=1 resident_bytes=9340860 disk_bytes=17825792 carrier_bytes=1048576 refusal_wall_ns=\d+ network_bytes=0$',text('control-14.txt'),re.M)
  core=json.loads(text('shared-core.json'));assert all(r['source']=='git+https://github.com/R4-Laboratories/SpiralCore?rev=8994951b7c13fe5acf6f5ea3060ad4e51dc473c2#8994951b7c13fe5acf6f5ea3060ad4e51dc473c2' for r in core)
  records.append({'target':target,'artifact':path.name,'sha256':artifact['sha256'],'members':len(z.namelist()),'source_files':len(hashes),'compiler_test_executable_sha256':checksum,'controls':observed,'scope':'native host-process fixtures, not booted MVM/WAN/capacity'})
if sys.argv[1:]==['--write-summary']:
 (root/'consumer-validation.json').write_text(json.dumps(records,indent=2)+'\n')
else:
 assert not sys.argv[1:]
 assert json.loads((root/'consumer-validation.json').read_text())==records, 'retained summary differs'
print(json.dumps({'platforms':len(records),'controls_each':15,'source_hash_rows':sum(r['source_files'] for r in records),'passed':45,'scope':'host-process filesystem/custody controls'}))
