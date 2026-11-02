# Local artifact cleanup

After the immutable Hugging Face ZIP was freshly downloaded, compared byte for
byte, extracted and all 30 manifest entries verified, only this task's owned
local experiment directory, archive staging directory, ZIP, download directory
and temporary download-path pointer were moved with `/usr/bin/trash`.

The experiment directory contained approximately 184 MiB, largely reproducible
public fixtures. Exact binaries, source snapshots and measurements are in the
verified archive; fixture identities and generation commands remain in Git and
the archive. Shared Cargo targets, other labs, prior experiments and user files
were untouched. Trash was not emptied: this clears the working directories but
does **not** claim permanent disk-space reclamation.

Owned paths were:
- `/Users/casey.allard/spiralcore/local/schema-context-cpu-20261102`
- `/Users/casey.allard/spiralcore/local/schema-context-cpu-20261102-archive`
- `/Users/casey.allard/spiralcore/local/schema-context-cpu-20261102.zip`
- `/private/tmp/spiralcore-hf-schema-context-QvCdFj`
- `/Users/casey.allard/spiralcore/local/schema-context-download-path.txt`
