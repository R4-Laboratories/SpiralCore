# MVM component failures retained

This isolated qualification source starts at MVM7d031c7 and overlays the owned
network changes. Access work is concurrently changing other files; this snapshot
does not qualify the entire current shared branch. Whole integrated validation
remains a final gate.

- policy-api-red: expected new types/accessors absent (E0432/E0599), before DTO
  implementation. Subsequent focused3tests and strict lib Clippy pass.
- plan-api-red: expected AdmittedNetworkPlan absent (E0425/E0433), before type
  implementation. Focused materialization test then passed.
- plan-forward-red: expected builder method absent (E0599), before forwarding.
  Focused materialization/config test then passed, including absent-None key.
- hook-api-red: not accepted as a hook API RED. Compilation first found three
  required mechanical None fields in pre-existing runtime literals (E0063).
  Their original output is retained; the actual local files were corrected in
  the lead-approved scopes.
- hook-api-red-after-forwarding: not accepted as a hook API RED. The isolated
  base-parent snapshot's mechanical patch also added the field to one unrelated
  WorkloadLaunchInputs literal (E0560). The actual shared parent is owned by the
  other contributor and was not altered by this patch. Diagnosis removed only
  that erroneous remote-snapshot field; the two real EndpointSpawnInputs fields
  remain. Do not claim either bootstrap failure proves endpoint behavior.

Next scoped check: compile the actual new hook test after these caller-field
fixes and retain its result. Do not implement the missing hook before observing
its genuine absence test. No VM, global process/history inspection or shared
target cleanup was performed.
