# ADR 0003: runner isolation

Status: accepted for implementation.

The domain depends on a runner port. Tests use deterministic fakes. The local
production-capable boundary uses pinned OCI artifacts, structured argv, no
network by default, non-root execution, read-only roots, quotas, bounded output,
timeouts, and explicit environment allowlists. A container is not described as
a complete hostile-code sandbox.
