# Original resident grant for the installed mailbox recovery job

The resident can now stage a separately typed V2 recovery job against the
exact signed plan, signer, and enrollment. It transfers the precharged
worker and original broker workspace into the same one-use handoff. The
legacy entry point cannot issue that job. The resident rechecks the grant
at worker entry and on both sides of delegated accept, and forwards the
original deadline. A refused post-accept check drops the candidate before the
service can use it. Runtime retains its normal shutdown custody.

The locked Linux release run passed all 10 resident configuration tests.
The new fixture presents a signed V2 plan, rejects the wrong enrollment and
legacy mode, issues one recovery job, forwards the exact deadline, and
retires the charged workspace. Its replacement comes from a Unix socket
fixture. Independent Code, Rust, and Security review passed. See
[patch](recovery-owner.patch) and [validation](validation.txt).

The V2 staging entry point is available to the qualification harness. Live
Hostd dispatch is still disabled. The next witness uses the existing signed
Firecracker mailbox fixture, binds the host and guest public pins and the
recovery session identifier to the signed enrollment, and redials the same
MAILBOX_PORT after a clean FIN. The new carrier must authenticate before the
resident replays the exact original Reply. The witness then verifies the
original final ACK, subsequent Cast, and resource retirement.
