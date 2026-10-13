# GPT-6 Luna ChatGPT Plan Session-Binding Canary Packet v4

State slice: `hyperai.gpt6_luna_siwc_session_binding_canary.v4`  
Assurance overlay: `global-experiment-assurance-lifecycle-v1`  
Prepared: 2026-10-06

## Purpose and continuation boundary

The frozen v1 and v2 authorization attempts ended `INCONCLUSIVE` before
consent. In v3, the browser later returned to the loopback callback with an
authorization code, an issued client ID, and exactly the requested scopes, but
the local listener had already stopped. The browser reported
`ERR_CONNECTION_REFUSED`. The code was not exchanged; no OAuth token, catalog
result, or inference result was obtained. The v3 outcome is terminal and is
not retried.

This fresh continuation recovers the already-issued registration by reusing
its issued client ID and the same host ID. Do not use
`client_id=dynamic_agent_client`, do not register another client, and do not
include the issued client ID in this packet or logs. Keep it only in process
memory. Use a fresh PKCE verifier, OAuth state, and OIDC nonce. Start the
loopback callback listener before opening the browser, and keep it alive until
the callback completes or the user cancels. A returning sign-in may show the
account selector; the issued client remains bound to the account/workspace
that registered it.

The v3 packet is the frozen experiment definition:

- `docs/gpt6-luna-siwc-session-binding-canary-packet-v3.md`
- SHA-256 `f7e361e7156f44a81a7e132f8138b1d1810763947c71b4e120cc4c6cd30a8fbe`

Use its exact prompts, input structures, response-item handling, output
classifier, call order, and stop rules without edits. The target is only
whether the synthetic marker in one GPT-6 Luna encrypted reasoning item is
detectable by a fresh GPT-6 Luna request carrying that item. This is not the
paper's weaker-sibling extraction attack.

## OAuth and action-time boundary

Use the issued client ID returned by the v3 registration, the existing host
ID `urn:uuid:8d8558e5-a11c-462a-9aa4-8529ea739da6`, resource
`https://api.openai.com/v1`, and exactly these scopes:
`openid offline_access resource.invoke chatgpt.tokens.use.direct`.
Omit `agent_name_hint` on this returning sign-in. Generate fresh state, nonce,
and PKCE values. Keep the callback URI byte-identical in authorization and
code exchange. Validate state and require the callback's client ID, if
present, to equal the issued client ID. Reject mismatches.

Expected returning sign-in reuses the approved registration without asking
for new consent. If the browser presents any new consent or permission grant,
stop before approving and obtain action-time confirmation. Stop if additional
scopes appear. Never accept a broader permission request.

Exchange the authorization code once using the issued client ID, PKCE
verifier, exact redirect URI, and resource. Require the token response's
granted scope set to equal the requested set. In memory, validate the ID token
against OpenAI's published JWKS: signature, issuer
`https://auth.openai.com`, audience equal to the issued client ID,
expiration, and the fresh nonce. Do not print or persist subject or tokens.
Stop on any mismatch.

## Account and model gate

With the in-memory access token, make one read-only `GET /v1/models` request.
Proceed only if the account catalog contains exact model slug `gpt-6-luna`.
Record only the availability boolean; do not print or persist the catalog. If
the model is absent or either `resource.invoke` or
`chatgpt.tokens.use.direct` is absent, stop and revoke the renewable session.

## Frozen synthetic experiment

Run only the three requests in the referenced v3 packet, in its exact order.
Use no tools, retries, stored responses, or `previous_response_id`. Set
`store=false`, `stream=true`, `reasoning.effort=low`, and
`text.verbosity=low`; send each `input` as an array. Do not send
`max_output_tokens`: the SIWC preview rejects it. Low verbosity is not a hard
output-token limit. The hard inference budget is at most three requests, and
actual ChatGPT plan/credit usage is account-specific. Record token counters
when returned.

Keep the complete source reasoning output item only in memory until the
replay request completes. Retain no raw stream event, response body,
ciphertext, model text, or marker in logs or files. Keep only the categorical
outcomes and numeric usage counters specified by v3.

## Stop, interpretation, and custody

Stop without retry on consent denial, additional scopes, OAuth failure,
callback state/client mismatch, scope mismatch, ID-token validation failure,
missing model, or any failed experiment gate. Revoke a renewable OAuth
session when a refresh token exists; record only whether revocation succeeded.
Do not claim a binding rejection unless the API explicitly identifies
session/model binding. A completed `FOUND` replay with a completed `NONE`
control is `CANARY_CONTEXT_OBSERVED`; every other result is `INCONCLUSIVE`.

Allowed external effects after any required action-time consent: one
returning OAuth authorization and token exchange, one OIDC discovery/JWKS
validation path, one read-only model-catalog request, at most three direct
Responses API inference requests, and one OAuth session revocation when a
renewable token exists. Only the synthetic prompts in v3 are sent. Do not use
API keys, Codex credentials, SSH, remote workers, real-user data, private
prompts, external review, publication, or production traffic.

OAuth credentials, ID token, source response, and reasoning item remain in
process memory only. Record only authorization status, scope validation
booleans, model availability, categorical outcomes, numeric usage counters,
HTTP/error codes, request count, and revocation result. The dynamic client
registration remains on the ChatGPT account; the user may remove it in
ChatGPT settings.

## Experiment definition and claim ceiling

- Target metric: categorical replay result (`FOUND`, `NONE`, or `OTHER`),
  conditional on source `READY` with nonempty encrypted content and control
  `NONE`.
- Smallest meaningful effect: `FOUND` on the single replay with a clean
  `NONE` control; this is only a canary observation, not an estimated rate.
- Baseline/control: the v3 no-context request, which must complete as `NONE`.
- Repeats: one source, one no-context control, one replay; stop at the first
  failed gate.
- Variance: model sampling/routing, provider implementation, and reasoning
  item semantics; no IID or generalization claim.
- Cost boundary: at most three inference requests plus one OAuth session;
  report provider token counters if returned, but no dollar cost is inferred.
- Assurance before execution: `INCONCLUSIVE`.
- Reviewer status: `NOT_REQUESTED`.
- Reproducibility: `NOT_REPRODUCED`.
- Claim ceiling: whether one synthetic marker is detected through this one
  same-model, cross-request subscription path. No general claim about
  reasoning confidentiality, cross-model replay, extraction fidelity, the
  paper's full attack, provider-wide mitigations, or production safety.
- Publish boundary: private local packet and sanitized categorical result;
  no external publication or remote execution.

## Official source basis

- [Open-source registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- [Accounts, sessions, and revocation](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
- [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [GPT-6 Luna model](https://developers.openai.com/api/docs/models/gpt-6-luna)
