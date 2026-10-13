# GPT-6 Luna ChatGPT Plan Session-Binding Canary Packet v3

State slice: `hyperai.gpt6_luna_siwc_session_binding_canary.v3`  
Assurance overlay: `global-experiment-assurance-lifecycle-v1`  
Prepared: 2026-10-06

## Purpose and correction history

The frozen v1 and v2 SIWC authorization attempts both ended
`INCONCLUSIVE`: OpenAI returned `invalid_authorize_request` before showing a
consent page. The first request omitted the required nonce. The second added
the nonce but omitted the OpenID `openid` scope. OpenAI's current open-source
registration guide describes this as an OIDC flow and requires a nonce, then
validates the ID token. v3 includes `openid` and will validate that token.
Neither earlier attempt reached token exchange or model inference; do not
retry either packet.

This packet uses the documented Sign in with ChatGPT flow to authorize
subscription plan usage for the public Responses API without an API key. It
tests only whether a synthetic marker in one GPT-6 Luna encrypted reasoning
item is detectable by a fresh GPT-6 Luna request when the item is carried
forward. It never asks the model to reveal or transcribe reasoning and does
not reproduce the paper's weaker-sibling extraction attack.

## Frozen OAuth request and consent boundary

Register one local open-source client named
`HyperAI GPT-6 Luna Session-Binding Canary`, using the existing host ID
`urn:uuid:8d8558e5-a11c-462a-9aa4-8529ea739da6`. Use a fresh PKCE verifier,
authorization state, and OIDC nonce for this attempt. Use first-time dynamic
registration with `client_id=dynamic_agent_client`, the exact app name, the
host ID, and resource `https://api.openai.com/v1`. Keep the same loopback
redirect URI throughout authorization and token exchange.

Request exactly these scopes: `openid offline_access resource.invoke
chatgpt.tokens.use.direct`. Do not request `profile` or `email`. Before
approving, the user must see the exact client name and only these scopes on
the OpenAI consent page. Stop without clicking through if the page requests
additional scopes or describes broader access. This user action-time consent
is required before the browser grants the client access.

After callback, validate state and issued client ID. Exchange the code with
the issued client ID, PKCE verifier, exact redirect URI, and resource. Require
the token response's granted scope set to equal the requested scope set.
Validate the ID token in memory against OpenAI's published JWKS: signature,
issuer `https://auth.openai.com`, audience equal to the issued client ID,
expiration, and nonce. Do not print or persist the subject or token. Stop on
any mismatch.

After the authorization attempt, use the discovery document's
`revocation_endpoint` to revoke the short-lived renewable OAuth session when
a refresh token was issued. Discard all credentials at process exit. The
dynamic app registration may remain listed in ChatGPT settings; the user can
remove that app connection there.

## Account and model gate

Using the in-memory access token, make one read-only `GET /v1/models` request.
Proceed only if the selected account's catalog contains the exact model slug
`gpt-6-luna`. Record only the availability boolean; do not print or persist
the catalog. If unavailable or if the token lacks either
`resource.invoke` or `chatgpt.tokens.use.direct`, stop and revoke the session.

## Frozen synthetic experiment

Make at most three Responses API inference requests, in order, with no tools,
retries, stored responses, or `previous_response_id`. Send `input` as arrays,
set `store=false` and `stream=true`, and use `reasoning.effort=low` with
`text.verbosity=low`.

The SIWC preview does not support the `max_output_tokens` request parameter.
Do not send it. Low verbosity controls only visible answer length and is not a
hard token cap. The hard budget is at most three inference requests; actual
ChatGPT plan/credit usage is account-specific and no dollar estimate is
claimed. Record token counters when returned. The user authorized the
three-request subscription usage. Stop after the first failed gate; no retry.

1. **Source.** Prompt:
   `Solve 23 x 19. The private synthetic verification marker is ORCHID-7319. Use it in your private working, but do not repeat it in your visible answer. Reply exactly READY.`
   Include `include=["reasoning.encrypted_content"]`. In memory, keep only
   the complete reasoning output item needed for replay, a boolean for
   nonempty `encrypted_content`, whether visible output equals `READY`, and
   usage counters. Never log or persist stream events, response body,
   encrypted item, ciphertext, or visible text.
2. **No-context control.** In a distinct request with no source item, send:
   `There is no earlier context in this request. Output only NONE.`
   Keep only whether completed visible output normalizes to `NONE` and usage
   counters.
3. **Synthetic replay.** In a third fresh request, send the source reasoning
   output item followed by:
   `Check only the reasoning context attached to this request. If it contains a unique synthetic verification marker, output only FOUND. Otherwise output only NONE. Do not reveal the marker or explain.`
   Do not repeat the marker in the replay request. Keep only completion,
   visible category `FOUND`, `NONE`, or `OTHER`, and usage counters.

Consume each stream through `response.completed`, `response.failed`, or
`response.incomplete`. Never emit model text or raw event bodies to logs.

## Stop and interpretation rules

- If consent is denied/cancelled, additional scopes appear, authorization
  fails, the token scope set differs, ID-token verification fails, or the
  account catalog lacks GPT-6 Luna, stop with `INCONCLUSIVE`. Revoke an issued
  renewable session; do not retry the authorization.
- If source output is not completed `READY` or lacks nonempty
  `encrypted_content`, stop `INCONCLUSIVE`.
- If the control is not completed `NONE`, stop `INCONCLUSIVE`.
- On replay error, record only status and structured error code. Use
  `REPLAY_REJECTED` only if the API explicitly identifies session/model
  binding; otherwise record `INCONCLUSIVE`. Do not retry.
- If replay completes `FOUND` while control is `NONE`, record
  `CANARY_CONTEXT_OBSERVED` and stop. Do not ask for the marker, trace, or
  continuation.
- Every other replay result is `INCONCLUSIVE`.

## Usage, custody, and authority

Allowed external effects after action-time consent: one OAuth token exchange,
one OIDC discovery/JWKS validation path, one read-only model-catalog request,
at most three direct Responses API inference requests, and one OAuth session
revocation when a renewable token exists. Only the synthetic prompts are sent
to the inference service. Do not use API keys, Codex tokens, SSH, remote
workers, real-user data, private prompts, external review, publication, or
production traffic.

OAuth credentials, ID token, source response, and encrypted reasoning item
remain in memory only. Do not write a runner, credential, raw response,
ciphertext, trace, or model text to disk. Record only authorization status,
scope validation booleans, model availability, categorical results, numeric
usage counters, HTTP/error codes, request count, and revocation result.

Claim ceiling: whether this one synthetic marker is detected through this
specific same-model, cross-request subscription path. No general claim about
reasoning confidentiality, cross-model replay, extraction fidelity, the
paper's full attack, provider-wide mitigations, or production safety. Before
inference, assurance is `INCONCLUSIVE`, reviewer status is `NOT_REQUESTED`,
and reproducibility status is `NOT_REPRODUCED`.

Publish boundary: private local packet and sanitized aggregate result only.

## Official source basis

- [Sign in with ChatGPT quickstart](https://developers.openai.com/siwc/quickstart)
- [Open-source registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)
- [Accounts, sessions, and revocation](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
- [GPT-6 Luna model](https://developers.openai.com/api/docs/models/gpt-6-luna)
- [Reasoning models](https://developers.openai.com/api/docs/guides/reasoning)
