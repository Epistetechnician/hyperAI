# ADR 0002: persistence and events

Status: accepted for implementation.

Use normalized current-state repositories plus append-only audit events and a
transactional outbox. Development may use SQLite and filesystem CAS behind
ports compatible with PostgreSQL and S3-style content addressing. Frozen specs,
evidence, attestations, resolutions, and audit records are append-only.
