# Private directories for installed Cast workers

Each original worker reads a signed boot grant from its VM directory. On Linux with umask `0002`, the original publication test created group-writable role directories. The reader refused the first signed grant. The installed SDK also used ordinary directory creation, so an operator’s umask could produce the same permissions. The failing Linux control and independent reproduction are retained in the [test record](private-state.json).

The two-worker Cast profile now creates a fresh per-VM directory with mode `0700` under the original VM name claim. It checks ownership and write permissions and rejects symbolic links before stopped-state cleanup. For long socket paths, MVM places its Unix sockets under a hashed name in `/tmp`. The SDK claims that name exclusively under the root-owned sticky directory. If the name already exists it leaves the contents intact and refuses the launch. After native admission begins, the SDK’s original recovery path owns both directories. [Runtime implementation](https://github.com/Casey-allard/mvm/blob/62c6f6eedee19fa91089720c12d1941a0b67dac3/crates/mvm-client/src/launch/cast_state.rs)

| Platform | umask | Private VM and split socket directory tests | Signed publication test |
|---|---|---:|---:|
| Linux x86_64 | `0002` and `0077` | 2 passed per mask | 1 passed per mask |
| macOS arm64 | `0002` and `0077` | 2 passed per mask | 1 passed per mask |

The signed-publication test includes a group-writable refusal after a successful check of both original boot files. The two SDK tests exercise a writable parent or leaf, a symbolic link, an occupied short socket namespace and rollback of fresh empty directories. Reproduce each row at the recorded MVM revision by setting the shell umask, then running `cargo test --locked -p mvm-client --lib launch::cast_state::tests::` and `cargo test --locked -p mvm-hostd --lib pair_publication_rejects_missing_stale_and_replaced_original_sidecars`. These component runs booted zero guests. [Results and executable hashes](private-state.json)
