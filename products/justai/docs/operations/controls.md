# Operations controls

State slice: `justai-operations-v1`.

Operations are local and authority-negative. The broker journal rejects replayed attempt identifiers, chains receipt digests, requires reversible operations, and fixes `accepted_evidence_write=false`. The subprocess harness requires a fresh directory, observable stdout/stderr digests, an outcome, and `accepted_evidence_write=false`.

Manual fallback is explicit: ambiguous rollback, infrastructure uncertainty, missing custody, or changed provenance produces `quarantine` with preserved alternatives. Recovery can restore only an unambiguous known baseline; it cannot invent a baseline, promote evidence, or authorize an external action.

Mesh-derived infrastructure semantics remain separate: infrastructure reachability, connector health, authentication, application correctness, and deployment state are independent status dimensions. A reachable host is not a healthy connector, authenticated operator, correct application, bound deployment, or authorization.
