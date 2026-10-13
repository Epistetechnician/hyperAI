# Local quorum verification v1

`justai-local-quorum-v1` permits repeated verification by machines or instances controlled by the same operator. It replaces the external-review prerequisite for the local governed decision loop, but remains a reproducibility and engineering-evidence lane, not an independent-review lane.

The packet requires at least three signed instance attestations, at least two distinct machine digests, matching observed-result digests, bounded execution timestamps, and the fixed claim ceiling `local-replay`. Raw artifacts and private locators never enter the packet.

Verify a packet with:

```text
python3 tools/local_quorum.py /approved/local-quorum/packet.json
```

The verifier always reports `publication_eligible=false`. Local quorum can support local governed promotion at the fixed `local-replay` ceiling, but cannot establish external custody or upgrade a claim to scientific evidence, safety proof, production authorization, or political authority. Public release and protected-intake policy remain separate gates.

The instance signatures bind this exact payload shape:

```text
justai:justai.local_quorum.v1:<packet_content_digest>:instance:<instance_id>
```

The machine digest is an attestation reference, not proof that the machine was independently controlled. The operator must retain the execution logs and any required platform attestations under the applicable custody regime.
