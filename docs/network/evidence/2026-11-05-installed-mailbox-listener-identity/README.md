# Revalidate the original installed mailbox listener after boot

Boot readiness still requires the existing accepting flag before and after
the original directory/socket checks. The new identity-only query uses the
same pinned directory and socket, and verifies the original owner's lifetime.
It changes no readiness or accept state.

The locked Core host-lifetime run passed twelve tests. The new Linux Unix
socket regression verified the original entry after readiness was revoked,
refused a substitute at the same pathname, accepted the original entry when
restored, and refused after original owner loss. The original boot-ready
query stayed refused. Independent Code, Rust and Security review passed.

This development patch does not rearm the listener. The combined integration
needs a separate typed job and original-worker one-use accept control.
