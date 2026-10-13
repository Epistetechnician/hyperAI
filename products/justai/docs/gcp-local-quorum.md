# GCP local quorum runner

The `justai-local-quorum-v1` GCP runner provisions exactly three short-lived Debian 12 `e2-micro` instances in `us-east4-a`, `us-east4-b`, and `us-east4-c`. Instances have no external address, no service account, no OAuth scopes, and receive only a synthetic public input. The guest startup script derives the artifact and result digests, emits metadata-only fields on the serial console, waits for the packet digest, and signs that digest with an ephemeral Ed25519 key held only on the guest during the run.

Run it from the repository root:

```text
python3 tools/gcp_local_quorum.py
```

The command uploads only the validated metadata-only packet to the private staging bucket `justai-protected-intake-secops1219` under `local-quorum/<run-id>/packet.json`, verifies byte equality after upload, and deletes the exact three instances and their boot disks in a `finally` cleanup path. Compute is billable. The bucket is controlled by the operator and is not external custody. The terminal output must remain `claim_ceiling=local-replay` and `publication_eligible=false`.
