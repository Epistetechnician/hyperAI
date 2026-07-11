# References

Accessed 2026-07-10 unless stated otherwise. Primary sources govern
implementation claims; secondary sources are discovery aids only.

## Hyperliquid HIP-4

- [HIP-4 outcome markets](https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-4-outcome-markets)
- [Asset IDs](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/asset-ids)
- [Spot Info endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/spot)
- [Exchange endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint)
- [Signing](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/signing)
- [Nonces and API wallets](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/nonces-and-api-wallets)
- [Official Python SDK](https://github.com/hyperliquid-dex/hyperliquid-python-sdk),
  observed commit `2fdb18f9517675ea03695a0962bd19eece9c83f0`
- [Official Rust SDK HIP-4 draft PR 190](https://github.com/hyperliquid-dex/hyperliquid-rust-sdk/pull/190)

The official Spot Info page, last updated three days before access, documents
`outcomeMeta` with request `{"type":"outcomeMeta"}`, an `outcomes` array
response, and the label "testnet-only." Direct read-only observations on
2026-07-10 found `outcomeMeta` and `settledOutcome` returning HTTP 200 on both
official mainnet and testnet Info endpoints. The documentation/rollout mismatch
is capability uncertainty, not write authority or an SLA. The implementation
uses the documented list request and treats outcome IDs as network-scoped.

## Challenge corpora and harnesses

- [NYU CTF Bench](https://github.com/NYU-LLM-CTF/NYU_CTF_Bench)
- [NYU CTF agents](https://github.com/NYU-LLM-CTF/nyuctf_agents)
- [CTF Archives](https://github.com/sajjadium/ctf-archives)
- [pwn.college CTF archive](https://github.com/pwncollege/ctf-archive)
- [CryptoHack CTF archive repository](https://github.com/cryptohack/ctf_archive)
- [CryptoHack hosted archive](https://cryptohack.org/challenges/ctf-archive/)
- [josephsurin public challenge repository](https://github.com/josephsurin/my-ctf-challenges)
- [maple3142 public challenge repository](https://github.com/maple3142/My-CTF-Challenges)
- [alex-bellon public challenge repository](https://github.com/alex-bellon/ctf-challenges)
- [247CTF](https://247ctf.com/dashboard), manual metadata and recorded fixtures
  only without an official automation API and explicit authorization

## Research references

- [Autonomous LLM Agents and CTFs: A Second Look](https://arxiv.org/abs/2605.21497)
- [NYU CTF Bench paper](https://papers.nips.cc/paper_files/paper/2024/hash/69d97a6493fbf016fff0a751f253ad18-Abstract-Datasets_and_Benchmarks_Track.html)
- [CTF archive research](https://arxiv.org/abs/2512.01233)
- [NYU CTF Bench secondary discovery page](https://www.emergentmind.com/topics/nyu-ctf-bench),
  not authoritative for implementation facts
