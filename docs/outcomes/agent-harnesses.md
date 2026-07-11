# Agent harnesses

Agent generation and candidate truth are separate. The recorded harness exposes
only a solver-view manifest, exports candidate bytes and trace commitments, and
records reward/cost telemetry without using reward as resolution evidence.

`config/harness-profiles.json` pins NYU `nyuctf_agents` commit
`6bb4d2b437c09457adb7dba6f15074f57af83a1f` and the D-CIPHER,
single-executor, and NYU baseline entrypoints. Default mode is recorded trace;
live models are disabled. Every extracted candidate must enter the submission
ledger and pass the pinned HSAI challenge verifier.
