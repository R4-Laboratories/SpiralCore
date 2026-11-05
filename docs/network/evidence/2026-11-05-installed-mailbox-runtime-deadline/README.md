# Original mailbox Runtime poll deadline

This candidate adds a deadline parameter to the original worker's listener and
stop wakeup poll. V1 passes no deadline and keeps the existing wait. The
replacement path can use an absolute deadline. An interrupted poll recomputes
its remaining time and the stop wakeup wins when both descriptors are ready.

The Linux test harness compiles the exact candidate function and its tests
from the patch under Rust 1.97.1 with optimizations and the same libc dependency
as the shared target. It covers an idle timeout, ready before deadline, refusal
at deadline and Stop taking precedence over an actual queued UDS connection.
The independent Code, Rust and Security review matched the candidate source
hash. See [the test record](validation.txt).

The helper is staged independently of the ongoing combined Hostd build. The
future Runtime consumer still needs one-attempt ownership, an original hard
deadline, actual Firecracker peer verification, and Stop checks around accept
and descriptor publication. This fixture does not enable Recovery V2.
