# Multi-instance self-verification

State slices: `justai-local-quorum-v1`, `justai-infrastructure-v1`.

The authorized replacement for the local decision-loop external-review prerequisite is a bounded same-operator quorum. Three isolated instances run the fixed Python contract and boundary checks against one archive produced from the exact target commit. The packet requires three distinct instance IDs, two distinct machine digests, matching result digests, and one Ed25519 attestation per instance.

The fixed cloud test set is `tools.tests.test_gcp_self_verify`, `tools.tests.test_local_quorum`, `tools.tests.test_multi_instance_self_verify`, `tests.custody.test_boundaries`, and `tests.integration.test_governed_loop`, followed by Python compilation of the governed runners and application boundaries. The source-checkout-dependent manifest-shape test remains host-only because source repositories are intentionally never transferred to GCP. The host root gate remains responsible for the complete repository suite.

The GCP execution boundary is non-authorizing:

- use a dedicated VPC and no-public-IP instances;
- transfer only the target commit archive and runner, never source checkouts, credentials, datasets, weights, traces, or provider configuration;
- grant instances no service-account API scope;
- sign the packet digest only after all instance results agree;
- validate the final packet with `python3 tools/local_quorum.py`;
- retain only metadata and digests in the packet; keep raw logs under the applicable custody policy;
- remove the temporary instances and network after the run unless a separately approved retention policy says otherwise.

This lane can support local replay and engineering evidence at `local-replay`. It does not establish independent review, external custody, scientific evidence, production readiness, hosted enforcement, profitability, or authorization. The local-quorum packet never becomes publication-eligible.
