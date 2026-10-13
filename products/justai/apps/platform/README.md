# Platform application

State slice: `justai-platform-v1`.

`src/runtime.py` provides project-scoped in-memory storage, reproducible aggregate-only export, and a synthetic hosted status surface. Export contains project and schema counts, never record payloads. The hosted runtime explicitly cannot authorize external execution, policy, validators, or releases.
