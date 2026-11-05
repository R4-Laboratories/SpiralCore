# Recovery profile fails closed through the installed provider

The retained SpiralCore provider composes with the staged MVM SDK's new
signed Recovery V2 profile. The existing V1 and ordinary branches retain
their behavior. The new profile refuses at the first provider match, before
original grant lookup, resource charging, or mailbox enrollment.

The test synthesizes and signs a bundled V2 plan with an exact enrollment,
admits it through the canonical Hostd checks, then invokes the original
provider directly. It verifies the named refusal, unchanged managed resource
usage, and absence of a staged enrollment. The exact composed RED exited 101
with the missing enum arm. The successor passed nine installed parent config
tests, including the existing original mailbox cases. Exact source review
passed.

This patch is based on the retained broker/expiry development composition.
It gives the next SDK and Runtime integration a fail-closed parent boundary.
Recovery V2 remains closed to actual dispatch until the original worker can
accept and authenticate a single replacement on the existing listener.
