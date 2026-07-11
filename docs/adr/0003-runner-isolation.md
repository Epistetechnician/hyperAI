# ADR 0003: runner isolation

Status: accepted for implementation.

The domain depends on a runner port. Tests use deterministic fakes. The local
production-capable boundary uses pinned OCI artifacts, structured argv, no
network by default, non-root execution, read-only roots, quotas, bounded output,
timeouts, and explicit environment allowlists. A container is not described as
a complete hostile-code sandbox.

Docker verification environments have a canonical
`hsai.docker-environment-binding.v1` identity. The binding commits to the
repository and immutable image digest, explicit Linux platform, UID,
structured argv, exact environment names and values, isolation flags, resource
controls, fixed input destinations, and environment-wrapper version. Its digest
must equal the claim's frozen `environment_digest` before input temporary files
are created or Docker is invoked. A changed image, platform, environment value,
or control therefore requires a new claim identity.

This binding prevents operator-selected environment drift. It does not qualify
the Docker host, daemon, kernel, default seccomp policy, or image contents as a
complete hostile-code sandbox.
