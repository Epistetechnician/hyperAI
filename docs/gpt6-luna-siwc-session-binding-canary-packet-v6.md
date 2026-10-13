# GPT-6 Luna ChatGPT Plan Session-Binding Canary Packet v6

State slice: `hyperai.gpt6_luna_siwc_session_binding_canary.v6`  
Assurance overlay: `global-experiment-assurance-lifecycle-v1`  
Prepared: 2026-10-06

## Purpose and continuation boundary

The v4 returning sign-in and the v5 new registration both passed token-scope
and ID-token validation, but their one model-catalog requests did not list
`gpt-6-luna`. Both sessions were revoked and neither sent an inference request.
The user identified the tested account as the account active in this
conversation. This is user-supplied account attribution; no OIDC identity was
retained. v4 and v5 are terminal and will not be retried.

This packet prepares one check of the other saved ChatGPT account. The user
must confirm that account selection and any new consent at action time. Do not
select an account or authorize the registration before that confirmation. If
the other saved account is not the user's account or the user does not confirm
it, stop without registering a client.

Use first-time dynamic registration for the selected other account:
`client_id=dynamic_agent_client`, agent name
`HyperAI GPT-6 Luna Session-Binding Canary`, host ID
`urn:uuid:8d8558e5-a11c-462a-9aa4-8529ea739da6`, and resource
`https://api.openai.com/v1`. Generate fresh OAuth state, OIDC nonce, and PKCE
verifier. Start the loopback callback listener before opening the browser and
keep it alive until callback completion or cancellation. Do not reuse a
client ID issued to either earlier account registration.

The frozen experiment definition is the v3 packet:

- `docs/gpt6-luna-siwc-session-binding-canary-packet-v3.md`
- SHA-256 `f7e361e7156f44a81a7e132f8138b1d1810763947c71b4e120cc4c6cd30a8fbe`

Use its exact prompts, input structures, response-item handling, output
classifier, call order, and stop rules without edits. The target is only
whether a synthetic marker in one GPT-6 Luna encrypted reasoning item is
detectable by a fresh GPT-6 Luna request carrying that item. This is not the
paper's weaker-sibling extraction attack.

## Authorization and action-time boundary

Request exactly these scopes: `openid offline_access resource.invoke
chatgpt.tokens.use.direct`. Do not request `profile` or `email`. Before the
user's action, inspect the displayed app name and scope list. Ask for
action-time confirmation before the account selection that shares account
information or before approving a permission grant. Stop if the name differs,
the scopes differ, or the page describes broader access. Do not accept legal
terms through automation.

After confirmed authorization, validate OAuth state and require the callback's
issued client ID. Exchange the code once using that client ID, the fresh PKCE
verifier, exact redirect URI, and resource. Require the token response's
granted scope set to equal the requested set. In memory, validate the ID token
against OpenAI's published JWKS: signature, issuer
`https://auth.openai.com`, audience equal to the issued client ID,
expiration, and fresh nonce. Do not print or persist the subject or tokens.
Stop on mismatch.

## Account and model gate

With the in-memory access token, make one read-only `GET /v1/models` request.
Proceed only if its catalog contains exact model slug `gpt-6-luna`. Record
only the availability boolean; do not print or persist the catalog. If the
model is absent or either `resource.invoke` or
`chatgpt.tokens.use.direct` is absent, stop and revoke the renewable session.

## Frozen synthetic experiment

Only after every authorization and model gate passes, run the three requests
from the referenced v3 packet in their exact order. Use no tools, retries,
stored responses, or `previous_response_id`. Set `store=false`, `stream=true`,
`reasoning.effort=low`, and `text.verbosity=low`; send each `input` as an
array. Omit `max_output_tokens`, which the SIWC preview rejects. Low verbosity
does not impose a hard output-token cap. The hard inference budget is at most
three requests. Actual ChatGPT plan/credit use is account-specific; record
token counters when returned without inferring dollar cost.

Keep the source reasoning output item in memory only until replay completes.
Never log or persist raw stream events, response bodies, ciphertext, visible
model text, or the synthetic marker. Retain only the categorical results and
numeric usage counters defined in v3.

## Stop, interpretation, and custody

Stop without retry on account-selection or consent failure, additional
scopes, OAuth failure, callback state/client mismatch, scope mismatch,
ID-token validation failure, missing model, or any failed experiment gate.
Revoke the renewable session when a refresh token exists and record only
whether revocation succeeded. Do not claim a binding rejection unless the API
explicitly identifies session/model binding. A completed `FOUND` replay with
a completed `NONE` control is `CANARY_CONTEXT_OBSERVED`; all other results are
`INCONCLUSIVE`.

Allowed external effects after action-time confirmation and consent: one
user-selected other account, one new OAuth registration and code exchange,
one OIDC discovery/JWKS validation path, one read-only model-catalog request,
at most three direct Responses API inference requests to `gpt-6-luna`, and one
OAuth session revocation when a renewable token exists. Only v3 synthetic
prompts may be sent. Do not use API keys, Codex credentials, SSH, remote
workers, real-user data, private prompts, external review, publication, or
production traffic.

OAuth credentials, identity claims, source response, and reasoning item remain
in process memory only. Record only authorization status, scope-validation
booleans, model availability, categorical results, numeric usage counters,
HTTP/error codes, request count, and revocation result. The new client
registration may remain on the selected ChatGPT account; the user can remove
it in ChatGPT settings.

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
  implementation, and reasoning-item semantics; no IID or generalization
  claim.
- Cost boundary: at most three inference requests and one new OAuth
  registration/session; record token counters if returned, but do not infer
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
