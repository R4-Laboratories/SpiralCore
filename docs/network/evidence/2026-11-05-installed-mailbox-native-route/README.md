# Original Firecracker process observation for mailbox recovery

The launcher can mint one read-only route observer from the exact connected API
socket. The socket's peer PID must match the verified original guardian Ready.
The native process is pinned with the kernel's connected Unix peer pidfd,
and the launcher Child has a separate process observer. Later checks require
the original grant binding, original live native process, matching live socket
peer and original launcher. Stop and Drop revoke the observer.

The first direct-guardian prototype passed with a numeric native pidfd lookup.
Independent review identified a possible identity race if a scope/sudo wrapper
outlived the actual guardian. The successor obtains the pidfd directly from
each connected socket and refuses when the socket capability is unavailable.
It does not use a numeric PID reopen for socket identity.

The final locked guardian suite passed eight tests on Wick. The focused test
used a real guardian and native fixture subprocess with two Unix connections.
It refused a foreign peer and a different launch binding, accepted the exact
second connection while the original child lived, and refused it after exit.
The test verified the cached PID was still present on the original socket
when its pidfd signaled exit. The original guardian still held its lease until
its existing positive stop path. Exact Code, Rust and Security review passed.

The native fixture child is a test subprocess. The product must still obtain
the proof from Firecracker's actual connected API, bind the original bridge,
publish under Runtime's Stop fence, and authorize the enrolled guest session.
This adds no replacement authority to the current installed profile.
