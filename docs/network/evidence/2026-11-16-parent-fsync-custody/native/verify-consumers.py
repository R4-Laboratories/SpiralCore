from pathlib import Path
import hashlib,json,re,sys,zipfile
root=Path(__file__).resolve().parent
frozen={line.split('  ',1)[1].removeprefix('./'):line.split('  ',1)[0] for line in (root/'frozen-mvm-inputs.sha256').read_text().splitlines()}
assert len(frozen)==4758
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
prefix+'retention_slots_exist_before_first_reservation',
'supervisor::endpoint::flowmux::spiralcore_payload::pool_close_tests::generic_parent_retention_is_funded_before_operations_and_survives_drain',
'installed_endpoint::auxiliary_tests::store_retention_precedes_other_metadata_spending_and_cannot_be_reclaimed',
prefix+'installed_retention_tests::original_installed_metadata_permits_store_cleanup_after_admission_close',
prefix+'installed_retention_tests::original_installed_metadata_poison_retains_store_and_all_debits',
]
records=[]
for artifact in json.loads((root/'consumer-catalog.json').read_text()):
 path=root/(str(artifact['id'])+'-'+artifact['name']+'.zip');raw=path.read_bytes();assert len(raw)==artifact['bytes'] and hashlib.sha256(raw).hexdigest()==artifact['sha256']
 assert artifact['head']=='698c1fc5fe64ce0c82b5f1948ed740759853a19d'
 target=artifact['name'].removeprefix('native-store-consumer-').removesuffix('-1')
 with zipfile.ZipFile(path) as z:
  assert z.testzip() is None
  def text(name):return z.read(name).decode()
  assert text('source-and-tree.txt').splitlines()==['f2ddbcbf1039405698e1ac0dcb16e29a5bf3b676','165f5cc95fda4cba7da885e6546d22be8d3e94a5']
  hashes={line.split('  ',1)[1]:line.split('  ',1)[0] for line in text('source.sha256').splitlines()};assert hashes==frozen
  after=text('source-after.txt').splitlines();assert len(after)==4758 and all(line.endswith(': OK') for line in after);assert {line[:-4] for line in after}==set(frozen)
  before=text('executable-before.sha256');assert before==text('executable-after.sha256')
  checksum,executable=before.strip().split('  ',1);assert re.fullmatch('[0-9a-f]{64}',checksum)
  build=[json.loads(line) for line in text('build.jsonl').splitlines()]
  selected=[r for r in build if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='mvm_hostd' and r['target']['kind']==['lib'] and r['profile']['test'] and r.get('executable')]
  assert len(selected)==1 and selected[0]['executable']==executable and selected[0]['fresh'] is False
  assert 'host: '+target in text('rustc.txt').splitlines()
  assert text('stack-configured-kib.txt').strip()=='8192'
  linux='-linux-' in target
  selectors=expected+([prefix+'retirement_failure_tests::parent_fsync_refusal_after_rmdir_retains_original_lease_lock_and_debit'] if linux else [])
  assert f'ordinary_controls={18 if linux else 17}' in text('counts.txt')
  assert f'executed_controls={len(selectors)}' in text('counts.txt') and 'explicit_root_uid_controls=2' in text('counts.txt')
  assert {n for n in z.namelist() if re.fullmatch(r'control-\d+\.txt',n)}=={f'control-{i}.txt' for i in range(len(selectors))}
  if linux:
   assert 'parent-fsync-platform.txt' not in z.namelist()
  else:
   assert text('parent-fsync-platform.txt')=='NOT_RUN: Linux O_PATH parent-fsync refusal; no equivalent macOS gate claimed\n'
  observed=[]
  for i,selector in enumerate(selectors):
   log=text(f'control-{i}.txt');assert selector+': test' in text('test-list.txt').splitlines()
   assert 'test '+selector+' ...' in log
   summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log,re.M);assert summaries==[('ok','1','0','0')],(target,i,summaries)
   clocks={key:value for key,value in re.findall(r'^(real|user|sys)\s+([0-9.]+)$',log,re.M)};assert set(clocks)=={'real','user','sys'}
   if 10<=i<=14:assert re.search(r'STORE_FIXTURE_OBJECT exact_bytes=49 serialized_in_memory_request_bytes=268 serialized_in_memory_reply_bytes=218 verified_wall_ns=\d+ network_bytes=0',log)
   observed.append({'index':i,'selector':selector,'result':'1 passed/0 failed/0 ignored','time_p_seconds':clocks})
  assert re.search(r'^STORE_DROP_ORDER called=true ledger_held=true returned_credit_visible=false original_owner_released=true retirement_wall_ns=\d+$',text('control-6.txt'),re.M)
  assert 'explicit_unprivileged_late_cleanup_controls=1' in text('counts.txt')
  assert re.search(r'^FINAL_RMDIR_REFUSAL owner_links=0 original_lock_retained=true original_child_retained=true quarantined=1 active=1 resident_bytes=9340860 disk_bytes=17825792 carrier_bytes=1048576 refusal_wall_ns=\d+ network_bytes=0$',text('control-14.txt'),re.M)
  if linux:
   log=text('control-20.txt')
   assert re.search(r'^PARENT_FSYNC_REFUSAL errno_probe=9 first_cleanup_is_permit_drop=true child_links=0 owner_links=0 original_lock_retained=true original_lease_retained=true quarantined=1 active=1 resident_bytes=9340860 disk_bytes=17825792 carrier_bytes=1048576 cleanup_drop_wall_ns=\d+ network_bytes=0 scope=funded_pool_fixture_no_registered_child$',log,re.M)
   assert re.search(r'STORE_FIXTURE_OBJECT exact_bytes=49 serialized_in_memory_request_bytes=268 serialized_in_memory_reply_bytes=218 verified_wall_ns=\d+ network_bytes=0',log)
  core=json.loads(text('shared-core.json'));assert all(r['source']=='git+https://github.com/R4-Laboratories/SpiralCore?rev=8994951b7c13fe5acf6f5ea3060ad4e51dc473c2#8994951b7c13fe5acf6f5ea3060ad4e51dc473c2' for r in core)
  records.append({'target':target,'artifact':path.name,'sha256':artifact['sha256'],'members':len(z.namelist()),'source_files':len(hashes),'compiler_test_executable_sha256':checksum,'controls':observed,'scope':'native host-process fixtures, not booted MVM/WAN/capacity'})
if sys.argv[1:]==['--write-summary']:
 (root/'consumer-validation.json').write_text(json.dumps(records,indent=2)+'\n')
else:
 assert not sys.argv[1:]
 assert json.loads((root/'consumer-validation.json').read_text())==records, 'retained summary differs'
print(json.dumps({'platforms':len(records),'controls_each':{r['target']:len(r['controls']) for r in records},'source_hash_rows':sum(r['source_files'] for r in records),'passed':sum(len(r['controls']) for r in records),'scope':'host-process filesystem/custody controls'}))
