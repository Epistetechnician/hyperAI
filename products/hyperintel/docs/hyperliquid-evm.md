# Hyperliquid and EVM integration contract

## Correct division of responsibility

Hyperliquid is the financial substrate, not the inference transport.

```text
offchain encrypted inference
  -> deterministic signed evidence
  -> HyperEVM escrow, bonds, commitments, and settlement
  -> optional future HyperCore hedge over a mature external index
```

HyperCore provides onchain order books and clearing for supported financial
assets. It does not schedule GPUs, protect prompts, identify models, meter
tokens, or prove service delivery. HyperEVM provides general-purpose contract
execution in the same Hyperliquid state; contract calls still need explicit
authorization, replay protection, and application-level evidence.

Primary references:

- [HyperEVM overview](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperevm)
- [HyperCore order book](https://hyperliquid.gitbook.io/hyperliquid-docs/hypercore/order-book)
- [HyperEVM to HyperCore interaction](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/hyperevm/interacting-with-hypercore)
- [HIP-3 specification](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals)

The official documentation currently labels HyperEVM alpha and notes staged
availability for higher throughput and write-system-contract functionality.
Every release must re-verify deployed network capabilities instead of assuming
testnet documentation applies to mainnet.

The marketplace MVP has no mainnet dependency on CoreWriter or HyperCore read
precompiles. High-level and developer documentation currently describe
different rollout surfaces, and the developer page describes read precompiles
as testnet facilities.

## HyperEVM testnet contract set

### ProviderRegistry

- provider identity digest and settlement address;
- active signing and verification keys with activation and revocation;
- supported service-manifest commitments;
- bond-manager reference and status;
- no prompt, output, credential, or private evidence bytes.

### BuyerSessionEscrow

- buyer, session key, settlement asset, expiration, and maximum authorization;
- cumulative authorized amount and cumulative settled amount;
- nonce or monotonic sequence plus a domain-separated full-payload digest;
- buyer cancellation after bounded liabilities;
- permissionless escape after timeout.

### ProviderBondManager

- segregated provider collateral;
- per-service and aggregate exposure caps;
- bounded objective remedy schedule;
- challenge window and deterministic evidence requirements;
- withdrawal delay, pause, and finalization state.

### SettlementHub

- verifies buyer authorization and permitted provider destination;
- verifies monotonic cumulative amount and session constraints;
- enforces conservation among payout, fee, refund, and credit;
- commits route, usage, assurance, and settlement digests;
- emits events sufficient for independent reconstruction;
- cannot inspect prompts or decide subjective service quality.

### Governance and recovery

- role-separated multisig for upgrade, pause, registry, and treasury functions;
- timelocked upgrades after the beta exception window;
- bounded emergency pause that preserves refunds and escape;
- no single operator can both alter verification policy and seize escrow;
- published deployment bytecode, compiler settings, addresses, and release
  provenance.

## Payment flow

Prefer session authorization or cumulative vouchers to one transaction per
request:

1. Buyer funds or approves a capped session on HyperEVM.
2. Buyer signs a policy and monotonically increasing maximum authorization.
3. Execution occurs offchain and produces attributable receipts.
4. Settlement intent calculates the exact cumulative payable amount.
5. Operator or provider submits the latest valid cumulative voucher.
6. Contract pays only the delta, records the payload digest, and preserves the
   buyer's exit path.

This reduces gas and prevents replay without allowing the operator to increase
the buyer's cap.

The signed voucher domain binds chain ID, verifying contract, protocol and
schema version, session ID, buyer, provider and destination, settlement asset,
cumulative amount, expiry, policy digest, and offer digest. Contract state
rejects stale cumulative amounts, payload reuse, wrong domains, and finalized
sessions. These fields are mandatory defenses against cross-chain,
cross-contract, and cross-session replay.

## Dispute boundary

Automatic settlement accepts only objective contract-verifiable facts and
registered signature keys. More complex evidence may authorize a bounded
offchain reviewer or arbitration path, but cannot silently become an onchain
truth oracle. Subjective model quality remains a commercial support matter.

## HyperCore boundary

Do not send inference requests through CoreWriter. CoreWriter actions are for
documented HyperCore operations and have their own timing and account-state
constraints. HyperCore integration begins only after a non-tradable index has
passed its publication gates.

HIP-3 currently assigns the deployer responsibility for market definition,
oracle operation, leverage, and settlement, and documents a 500,000 HYPE
mainnet staking requirement that may change. HyperIntel must query and reapprove
those conditions at launch time. A hedge tracks index price risk; it does not
guarantee capacity delivery and does not inherit marketplace escrow collateral.

Official guidance also warns that most indices are unsuitable as perp oracles.
The operator design must handle approximately three-second oracle updates,
minimum update spacing, stale-price behavior, `haltTrading` settlement at the
current mark, manipulation resistance, and the irreversible consequences of
cross-margin enablement. Portfolio margin is currently alpha, and no documented
rule recognizes EVM escrow, provider bonds, or physical inference obligations
as HyperCore collateral.
