# GPT-6 Luna ChatGPT Plan Session-Binding Canary Packet v5

State slice: `hyperai.gpt6_luna_siwc_session_binding_canary.v5`  
Assurance overlay: `global-experiment-assurance-lifecycle-v1`  
Prepared: 2026-10-06

## Purpose and continuation boundary

The v4 attempt validated the returning OAuth session, but its account's single
read-only model-catalog response did not list `gpt-6-luna`. It stopped before
inference and successfully revoked its renewable session. v4 is terminal and
must not be retried. This packet checks one other ChatGPT account through a
new, account-bound SIWC registration.

Use the system account chooser and let the user select the account to test.
The selected account must differ from the account bound to the v4 registration.
Do not infer account identity from browser state, email, subscription labels,
or the account order. Do not retain or log the selected identity. If the user
does not select a different account, stop before registering a client.

This is a first-time dynamic registration for the selected account. Use
`client_id=dynamic_agent_client`, agent name
`HyperAI GPT-6 Luna Session-Binding Canary`, the existing host ID
`urn:uuid:8d8558e5-a11c-462a-9aa4-8529ea739da6`, and resource
`https://api.openai.com/v1`. Generate a fresh state, OIDC nonce, and PKCE
verifier. Start the loopback callback listener before opening the browser and
keep it alive until callback completion or cancellation. Never reuse the
v4-issued client ID for this different account.

The frozen experiment definition remains the v3 packet:

- `docs/gpt6-luna-siwc-session-binding-canary-packet-v3.md`
- SHA-256 `f7e361e7156f44a81a7e132f8138b1d1810763947c71b4e120cc4c6cd30a8fbe`

Use its exact prompts, input structures, response-item handling, output
classifier, call order, and stop rules without edits. The test asks only
whether one synthetic marker in one GPT-6 Luna encrypted reasoning item is
detectable by a fresh GPT-6 Luna request that carries that item. It does not
reproduce the paper's weaker-sibling extraction attack.

## Authorization and action-time boundary

Request exactly these scopes: `openid offline_access resource.invoke
chatgpt.tokens.use.direct`. Do not request `profile` or `email`. The user must
review the displayed client name and scopes. If consent or a permission grant
appears, stop before approval and obtain action-time confirmation. Stop if the
client name or requested scopes differ or if the page describes broader
access. Do not accept terms or a legal agreement through automation.

After user approval, validate OAuth state and require the callback's issued
client ID. Exchange the code once with that ID, the fresh PKCE verifier, exact
redirect URI, and resource. Require the granted scope set to equal the
requested set. Validate the ID token in memory against OpenAI's published
JWKS: signature, issuer `https://auth.openai.com`, audience equal to the
issued client ID, expiration, and fresh nonce. Do not print or persist the
subject or tokens. Stop on any mismatch.

## Account and model gate

With the in-memory access token, make one read-only `GET /v1/models` request.
Proceed only if its catalog contains exact slug `gpt-6-luna`. Record only the
availability boolean; do not print or persist the catalog. If unavailable or
if either `resource.invoke` or `chatgpt.tokens.use.direct` is absent, stop and
revoke the renewable session.

## Frozen synthetic experiment

Only after all authorization and model gates pass, run the three requests in
the referenced v3 packet, in exact order. Use no tools, retries, stored
responses, or `previous_response_id`. Set `store=false`, `stream=true`,
`reasoning.effort=low`, and `text.verbosity=low`; send each `input` as an
array. Do not send `max_output_tokens`, which the SIWC preview rejects. Low
verbosity is not a hard output-token limit. The hard inference budget is at
most three requests; actual ChatGPT plan/credit usage is account-specific.
Record token counters when returned.

Keep the complete source reasoning output item only in memory until replay
completes. Never log or persist a raw stream, response body, ciphertext,
visible model text, or marker. Retain only the categorical results and
numeric usage counters defined in v3.

## Stop, interpretation, and custody

Stop without retry on wrong account selection, consent denial, additional
scopes, OAuth failure, callback state/client mismatch, scope mismatch,
ID-token validation failure, missing model, or any failed experiment gate.
Revoke the renewable OAuth session when a refresh token exists and record
only whether revocation succeeded. Do not claim a binding rejection unless
the API explicitly identifies session/model binding. A completed `FOUND`
replay with a completed `NONE` control is `CANARY_CONTEXT_OBSERVED`; every
other result is `INCONCLUSIVE`.

Allowed external effects after action-time consent: one account selection and
new OAuth registration, one code exchange, one OIDC discovery/JWKS validation
path, one read-only model-catalog request, at most three direct Responses API
inference requests to `gpt-6-luna`, and one OAuth session revocation when a
renewable token exists. Only the synthetic prompts in v3 may be sent. Do not
use API keys, Codex credentials, SSH, remote workers, real-user data, private
prompts, external review, publication, or production traffic.

OAuth credentials, ID token, source response, and reasoning item remain in
process memory only. Record only authorization status, scope-validation
booleans, model availability, categorical outcomes, numeric usage counters,
HTTP/error codes, request count, and revocation result. The new registration
will remain on the selected ChatGPT account unless the user removes it in
ChatGPT settings.

## Experiment definition and claim ceiling

- Target metric: categorical replay result (`FOUND`, `NONE`, or `OTHER`),
  conditional on source `READY` with nonempty encrypted content and control
  `NONE`.
- Smallest meaningful effect: `FOUND` on the single replay with a clean
  `NONE` control; this is a canary observation, not an estimated rate.
- Baseline/control: the v3 no-context request, which must complete as `NONE`.
- Repeats: one source, one no-context control, one replay; stop at the first
  failed gate.
- Variance: model sampling/routing, account/workspace eligibility, provider
  implementation, and reasoning item semantics; no IID or generalization
  claim.
- Cost boundary: at most three inference requests plus one new OAuth
  registration/session; report token counters if returned, but do not infer
  dollar cost.
- Assurance before execution: `INCONCLUSIVE`.
- Reviewer status: `NOT_REQUESTED`.
- Reproducibility: `NOT_REPRODUCED`.
- Claim ceiling: whether one synthetic marker is detected through this one
  same-model, cross-request subscription path for the selected account only.
  No general claim about reasoning confidentiality, cross-model replay,
  extraction fidelity, the paper's full attack, provider-wide mitigations, or
  production safety.
- Publish boundary: private local packet and sanitized categorical result;
  no external publication or remote execution.

## Official source basis

- [Open-source registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- [Accounts, sessions, and revocation](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
- [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [GPT-6 Luna model](https://developers.openai.com/api/docs/models/gpt-6-luna)
