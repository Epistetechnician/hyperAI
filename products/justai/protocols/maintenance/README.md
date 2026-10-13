# Maintenance protocol

State slice: `justai-operations-v1`.

Maintenance uses bounded operations, receipt chains, replay protection, rollback to known baselines, subprocess quarantine, and manual fallback. It cannot write accepted evidence or silently change a policy, evaluator, signer, release authority, or kill path.
