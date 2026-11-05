# Firecracker guardian preflight under a systemd scope

Two isolated traced repeats reproduced the original installed failure with a verified private umask. The scope launcher reopened the original directory and holder descriptors, and the shared holder lock succeeded. The live SHA-256 output matched both the rendered expectation and the retained original state. The shell then exited with status 1 before invoking the privileged guardian, causing the original controller to observe EOF.

The exact shell preflight replay on a private copy of the retained synthetic fixture reproduced the failure under Wick's systemd 259 user scope. Systemd expanded the braced Bash digest expression before executing Bash. The one-line candidate compares the complete output from the fixed GNU sha256sum invocation. It removes the problematic expansion syntax while preserving the existing descriptor, holder, ownership, length, marker and digest checks.

The candidate preflight accepted valid state in the same scope. It refused both a one-byte, same-length change and a pending transaction marker. Independent scoped source review confirmed the original holder handoff remains intact. These are executed shell controls. The rebuilt product and an installed guest run are separate validation steps.
