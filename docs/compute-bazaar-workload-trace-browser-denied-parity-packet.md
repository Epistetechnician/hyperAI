# Workload Trace Actual Browser Denied-State Parity Packet

Status: `INCONCLUSIVE`; the actual served browser and filesystem-denied state
were not exercised. The browser failure and cleanup are recorded below.

State slice: `hyperai.bazaar.workload_trace_browser_denied_parity.v1`.
Parent: [Compute Bazaar Rust port subplan](compute-bazaar-rust-port-subplan.md).
Prior local evidence: [denied-state client parity](compute-bazaar-workload-trace-client-denied-parity-packet.md)
and [Linux reproduction](compute-bazaar-workload-trace-denied-linux-reproduction-packet.md).
Baseline: HyperAI `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa` on `main`, plus
the current dirty and untracked worktree. Preserve unrelated work.

## Frozen contract and inputs

Exercise the production browser shell against the real local Workload Trace
route and one HyperAI-authored synthetic JSON file. Start the current
`hyperai-ui` binary on `127.0.0.1:45239` with the disposable home
`/tmp/hyperai-browser-denied-parity-20260928`. Use a separately launched,
headless Chromium process with a new temporary profile and DevTools Protocol
connection bound to IPv4 loopback. Do not use or inspect an existing browser
profile, the user's in-app browser tabs, or the user's open Brave session.
Run Chromium under a generated macOS Seatbelt profile that denies all IP
networking except outbound to the test service at `127.0.0.1:45239`, plus
inbound and bind operations on `127.0.0.1:45241` for DevTools. Permit Unix
domain socket bind/connect below the private run-owned browser temp root and
only under the `com.google.Chrome.*` socket directory Chromium creates beneath
macOS's per-user temporary directory. Keep these exceptions limited to Unix
domain socket operations. Set `TMPDIR` to the private run-owned root for other
browser scratch files. Also use loopback-only host resolver rules and disable
background network features. Inspect DevTools request events and process
sockets, and stop immediately if any non-loopback connection is attempted.
Keep the existing service on `127.0.0.1:4317` untouched.

In the browser, create the fixed HyperAI synthetic Data snapshot through the
existing Data workspace action. Confirm the normal trace links the local Data
snapshot. Remove read bits from only
`/tmp/hyperai-browser-denied-parity-20260928/data/synthetic-market-observations.json`
and immediately probe a direct read. If that probe returns
`PermissionDenied`, refresh the ordinary Workload Trace from the UI and
require the served browser to display `denied` with no source digest and no
buyer-intent Data reference. Confirm the owner route reports the same state
and that the browser does not expose the local path or bytes. Restore the
original file mode in all exit paths, verify the readable control state, then
stop the test service and Chromium process group and remove its home and
browser profile.

The browser's fixed `Preview denied Data reference` control is explicitly a
display fixture and performs no Data read. Do not use it as evidence for this
actual filesystem-denied result.

## Browser access diagnostic

On 2026-09-28 the dedicated in-app browser returned
`net::ERR_BLOCKED_BY_CLIENT` for both `127.0.0.1:45239` and
`localhost:45239`; neither request reached the service. Chromium defines this
as a client-side request block without identifying which client policy caused
it. Repeating either navigation is out of scope. The supported fallback is a
separate headless Chromium process controlled through the local DevTools
Protocol, with a fresh profile and an OS policy limiting IP networking to the
two required loopback ports and limiting Unix-domain sockets to the private
run root plus Chromium's per-user temporary socket directory. The first
isolated launch stopped before page load because the process singleton socket
was created in that macOS per-user directory; setting `TMPDIR` alone did not
redirect it. Chromium documents creating its singleton socket in a temporary
directory, and Apple documents the per-user temporary-directory mechanism.
See the [Chromium ProcessSingleton source](https://chromium.googlesource.com/chromium/src/%2B/HEAD/chrome/browser/process_singleton_posix.cc),
[Apple temporary-directory guidance](https://developer.apple.com/library/archive/documentation/Security/Conceptual/SecureCodingGuide/Articles/RaceConditions.html),
and [Chromium
error definition](https://chromium.googlesource.com/chromium/src/%2B/refs/tags/143.0.7499.47/net/base/net_error_list.h),
[Chrome Headless documentation](https://developer.chrome.com/docs/chromium/headless),
and [DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/).

Included source inputs are read-only: `apps/hyperai-interface/app.js`,
`apps/hyperai-interface/workload-trace.js`, the current interface HTML/CSS,
`crates/hyperai-local-service/src/server.rs`, and the existing synthetic Data
seed implementation. Only this packet, the state-slice ledger, and the Rust
port subplan may be edited. No service, API, UI, schema, test, fixture, or
product behavior change is in scope.

## Allowed effects and proof gates

Allowed effects: build the existing UI binary with the pinned offline Rust
toolchain; run one disposable IPv4-loopback service on the preflighted port;
create only the fixed synthetic JSON/Parquet data; temporarily remove and
restore read bits on the JSON file; read the local owner route; generate and
preflight a temporary Seatbelt network profile and private browser temp root;
operate one separately
launched headless Chromium process with a new temporary profile and a DevTools
endpoint bound to `127.0.0.1`; and retain short-lived local output for hashing
before cleanup.

Proof gates:

- Verify port 45239 is free and preserve PID 85094 on port 4317.
- Build with
  `sh scripts/with-pinned-rust.sh cargo build --locked --offline -p outcomes-cli --bin hyperai-ui`.
- Capture the direct OS permission result. Require the actual browser and
  owner route to report the same `denied` state only if it is
  `PermissionDenied`; otherwise record `NOT_EXERCISED`.
- Inspect the Seatbelt profile before launch: it must deny outbound, inbound,
  and bind networking by default and allow only the loopback service, DevTools
  operations, and Unix-domain sockets under the private browser temp root or
  the `com.google.Chrome.*` subdirectory in the current user's macOS temp
  directory. Preflight only those local connections and private-root sockets;
  do not probe an external address.
- Verify the Chromium executable/version and fresh profile; bind DevTools only
  to `127.0.0.1`; inspect DevTools request events and all Chromium process
  sockets; and stop and clean up if a non-loopback connection is attempted.
- Inspect the visible trace status, absent digest, absent buyer-intent Data
  reference, focus after refresh, browser console, and one desktop plus one
  narrow viewport. Do not claim browser-wide accessibility from this check.
- Restore the file mode, verify the normal readable result, stop only the
  test-owned service and Chromium process group, and remove the disposable
  home, browser profile, and temporary logs.
- Run `corepack pnpm run lint`, `python3 scripts/verify_imports.py`,
  `git diff --check`, and scoped trailing-whitespace checks.

No provider, seller, registry, federation, external agent, model, paid
execution, remote Fleet, wallet, chain, funds movement, public endpoint,
publication, or deployment. No request leaves loopback. Keep any bearer value,
full temporary path, or raw response body out of the packet.

## Attempt outcome (2026-09-28)

The in-app browser blocked both `127.0.0.1:45239` and `localhost:45239` as
`ERR_BLOCKED_BY_CLIENT` before either request reached the service. A separately
launched Google Chrome 153.0.8010.53 instance with a fresh profile passed the
Seatbelt preflights for loopback and the scoped Unix-socket exceptions, then
reached its local DevTools listener. Chrome's internal sandbox and its network
and GPU helpers repeatedly failed under the outer Seatbelt profile; the
browser exited with code 133 before a page result could be inspected. Crashpad
also attempted a report request, which Seatbelt denied. That stop condition
was honored. No DevTools page request, UI action, data seed, direct permission
probe, or owner-route comparison completed. Temporary browser data, the
per-run Chrome socket directory, and the local test home were removed. The
pre-existing service on port 4317 remained listening as PID 85094. Browser
output was not retained as a digest-bound artifact because no browser result
was produced.

## macOS Seatbelt selector correction

The installed Google Chrome 153 unified headless binary is available; a
separate `chrome-headless-shell` binary is not installed in the local browser
caches. Downloading or installing a browser is outside this local-only slice.
The one authorized retry did not launch: macOS rejected the numeric host in
the generated Seatbelt rule with `host must be * or localhost in network
address`. This happened during profile parsing, before the service or browser
started. Research of the current Codex Seatbelt implementation confirms that
it uses `localhost:<port>` rules; macOS does not accept a literal IP selector
in this rule form. A port-specific `localhost` selector is the narrowest
available Seatbelt expression, but it does not itself prove that a socket's
numeric destination is `127.0.0.1`. The new continuation slice below therefore
keeps every intended endpoint bound to IPv4 loopback, maps browser names to
loopback, inspects all browser sockets, and stops on any non-loopback socket.
It keeps all other destinations denied and does not probe external addresses.

No Chrome retry is authorized under the original numeric-address slice. The
new, separately named continuation slice permits one retry with the existing
Chrome binary and a fresh profile, using `localhost:45239` and
`localhost:45241` Seatbelt selectors. It adds `--disable-gpu`,
`--disable-crash-reporter`, and
`--disable-features=NetworkServiceSandboxMac` to avoid the GPU process,
Crashpad, and the nested macOS network-service sandbox implicated by the
previous code-133 exit. Chromium's macOS field-trial configuration lists
`NetworkServiceSandboxMac` as a macOS feature; that does not prove the feature
was active in this particular binary, so the flag is a bounded diagnostic.
Keep Chrome's renderer sandbox enabled. Inspect the exact policy before
launch and inspect DevTools requests, sandbox denials, and all browser-process
sockets during the run. Stop immediately on any non-loopback request, bind,
or socket attempt. If Chrome exits before a page result, record the gate as
`INCONCLUSIVE` and stop; another browser runtime or isolation change requires
another named slice.

Sources: [Chrome Headless mode](https://developer.chrome.com/docs/automation-and-testing/headless),
[Chromium macOS network-service sandbox configuration](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/testing/variations/fieldtrial_testing_config.json),
[Chromium headless crash-reporter switch](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/headless/public/switches.h),
and [Chromium GPU switch](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/content/public/common/content_switches.cc).

## Claim ceiling

The prior local Seatbelt preflight passed for the tested loopback service
port, the DevTools listener, and Unix-domain sockets under both approved
temporary locations. The numeric-address profile was later found not to be
representable in macOS Seatbelt and the controlled retry stopped during
profile parsing. The actual served browser and owner-route projection of a
filesystem-denied file remain `NOT_EXERCISED`; do not claim denied-state UI or
route parity. No Data snapshot was seeded and no file permissions were
changed. The attempt does not establish browser accessibility, cross-browser
behavior, external data, independent review, production readiness, or
external-effect authority.

Assurance: `INTERNAL_PASS` for the tested socket-policy preflight only;
actual browser gate: `INCONCLUSIVE`. Reviewer status: `NOT_REQUESTED`;
reproducibility: `NOT_REPRODUCED`. Custody is checked-out HyperAI source plus
the disposable test home/profile, all removed after the attempt. Actual
browser parity remains open pending the one retry authorized by
`hyperai.bazaar.workload_trace_browser_localhost_port_guard.v1` below.

No external request succeeded; the operating-system policy denied the
Crashpad report attempt. This is local diagnostic evidence only and does not
change the upstream import or marketplace plan.

Publish boundary: local documentation and evidence only; no commit, PR,
release, external circulation, or deployment.

## Localhost-port continuation slice

State slice: `hyperai.bazaar.workload_trace_browser_localhost_port_guard.v1`.
This continuation carries the same local synthetic purpose and source inputs,
but corrects the macOS Seatbelt selector within its supported grammar. Its
Seatbelt policy denies network access by default and allows only the two
named `localhost` ports plus the already scoped Unix-domain sockets. Because
macOS does not accept a numeric IP host selector, this rule cannot by itself
attest that a permitted socket uses the numeric loopback address. Therefore
the server and DevTools listener must bind only to `127.0.0.1`, Chrome must
resolve requests to `127.0.0.1`, and the live process socket inspection must
confirm every IP socket is loopback. Stop immediately on any other address,
request, or bind. No external address is probed; no provider, paid execution,
federation, or remote worker is involved.

The slice permitted exactly one browser retry with the already installed
Chrome binary and the flags above. It preserved the fresh profile, renderer
sandbox, background-networking disablement, and Unix-socket path limits. It
did not authorize a different browser, wider ports, relaxed default network
denies, application or test changes, or a second retry. Exact editable files
are this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

### Attempt outcome (2026-09-28)

The corrected deny-by-default profile parsed successfully. Port-limited
IPv4-loopback outbound, DevTools bind/inbound, and the scoped Unix-socket
exceptions passed local preflight. The disposable service started on
`127.0.0.1:45239`; Google Chrome `154.0.8037.58` exposed DevTools on
`127.0.0.1:45241`. The process-socket monitor observed only loopback IP
sockets. The Puppeteer controller then failed to fetch Chrome's DevTools
WebSocket URL before it created a page. No page request, UI action, Data seed,
permission change, or owner-route comparison completed. The browser request
monitor had no page requests to assess. The temporary service, browser
process group, profile, and test home were removed; the existing listener on
`127.0.0.1:4317` remained PID 85094.

Result: `INCONCLUSIVE`; actual filesystem-denied browser/route parity remains
`NOT_EXERCISED`. Assurance is `INTERNAL_PASS` only for profile parsing and the
scoped local socket preflights. Reviewer status is `NOT_REQUESTED`,
reproducibility is `NOT_REPRODUCED`, and no browser artifact was retained. The
SHA-256 of the sanitized runner summary emitted for this attempt is
`0c388e8d8b343c3a16b055493de3b3d6b75369fb4681a6354541e94b60084647`; it is a
summary digest, not a browser-output or screenshot digest. A further browser
attempt requires a new named slice.

## DevTools WebSocket continuation slice

State slice: `hyperai.bazaar.workload_trace_browser_cdp_websocket.v1`.
The prior port-guard slice is closed as `INCONCLUSIVE`. Local source inspection
shows Puppeteer 25.3's `browserURL` path performs its own global `fetch` of
`/json/version`, while the Python readiness probe reached that same
loopback-only endpoint. This continuation passes the WebSocket URL obtained
by that Python probe directly as Puppeteer's `browserWSEndpoint`, avoiding a
second controller-side HTTP fetch. Remove proxy environment variables from
the controller as well as Chrome.

This slice permits exactly one new Chrome launch with the same Chrome binary,
fresh profile, `localhost`-port Seatbelt rules, scoped Unix sockets, flags,
loopback resolver, and process-socket monitor defined above. It does not
relax the renderer sandbox or network policy, and it does not authorize a
second browser run, a different runtime, any application/test changes, or
external traffic. All synthetic Data seeding, filesystem permission changes,
UI comparisons, cleanup, and validation gates remain unchanged. Exact
editable files are this packet, `docs/state-slices.md`, and
`docs/compute-bazaar-rust-port-subplan.md`.

### Attempt outcome (2026-09-28)

The profile parsed, the service started on `127.0.0.1:45239`, and Google
Chrome `154.0.8037.58` returned its DevTools version endpoint over
`127.0.0.1:45241`. Python passed the returned WebSocket URL directly to
Puppeteer with proxy variables removed. The WebSocket connection then returned
`ECONNREFUSED` on `127.0.0.1:45241` before a page was created. The live socket
monitor observed only loopback IP sockets; no page request occurred. The
controller did not capture whether Chrome had exited when the refusal
occurred. No Data snapshot was seeded, no file permissions changed, and no
owner-route comparison ran. The service, Chrome process group, profile, and
test home were removed; PID 85094 on `127.0.0.1:4317` was preserved.

Result: `INCONCLUSIVE`; actual denied-state browser/route parity remains
`NOT_EXERCISED`. Assurance is `INTERNAL_PASS` only for profile parsing,
service readiness, the loopback DevTools endpoint response, and scoped socket
preflights. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. The sanitized runner-report SHA-256 is
`8308a4d9d424c2b5bf92561dac142ad5a4e99dfefb9a567236cad3a466e5b2fc`; the
service-log SHA-256 is
`001d4a1615206b7f5c05364681abd126b0142c6b45cfc3ba579b312bc9920ead`; and the
Chrome-log SHA-256 is
`aa48f200c15c22eaa0fefa2f432c28644974f98a1915671c2ae5c184e5b43d0d`. The
logs were removed after hashing; no screenshot or browser output was retained.
A further diagnostic or browser attempt requires another named slice.

## Chrome DevTools liveness diagnostic slice

State slice: `hyperai.bazaar.workload_trace_browser_chrome_liveness.v1`.
The prior direct-WebSocket slice is closed as `INCONCLUSIVE`. This diagnostic
permits one installed-Chrome launch using the same deny-by-default
port-limited Seatbelt policy, but it does not open the HyperAI page, seed Data,
change permissions, or invoke Puppeteer. It polls the loopback DevTools
version endpoint while also recording Chrome process liveness and local
socket addresses, then hashes and removes the temporary log. It is intended
to determine whether the listener disappears before the Puppeteer connection
attempt. No external address is probed. A later served-page run requires a
new named slice.

### Diagnostic outcome (2026-09-28)

The profile parsed and Google Chrome `154.0.8037.58` launched on
`about:blank`. The local DevTools endpoint on `127.0.0.1:45241` responded once;
about 0.45 seconds later it stopped responding and the Chrome process exited
with signal 5 (`SIGTRAP`). The socket monitor observed only loopback IP
sockets. No HyperAI service or page was opened, no Data was seeded, and no
file permission changed. The temporary browser profile and log were removed;
the log SHA-256 is
`b075bc3861ef8bdd9762750c6045badb60d6f00d790f3973d39bfb115aa9551d`. The
disposable home and listeners on ports 45239 and 45241 are absent; PID 85094
on `127.0.0.1:4317` remains preserved.

Assurance is `INTERNAL_PASS` only for profile parsing and the local socket
check. Chrome liveness and actual served-browser parity remain `INCONCLUSIVE`
and `NOT_EXERCISED` because the process exited before the five-second window
completed. Reviewer status is `NOT_REQUESTED`; reproducibility is
`NOT_REPRODUCED`. The observed `SIGTRAP` makes Chrome startup under the outer
Seatbelt profile the current local blocker. A different isolation profile or
browser runtime requires a new named slice.

After recording this outcome, `corepack pnpm run lint` passed; the pinned
import verifier passed for Simple Compute Market, HyperIntel, and JustAI;
`git diff --check` and the scoped trailing-whitespace check passed. These root
and import checks do not resolve the unexercised browser gate.
