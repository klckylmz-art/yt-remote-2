# Direct native v3 development validation — 2026-11-10

Status: **EXPERIMENTAL / NOT RELEASE READY / NOT NoMachine-equivalent**.
Work starts from verified CI branch revision `962b9e4` (PR #17); this change
does not merge or duplicate the older STOP/security/RustDesk policy PRs.
No Issue #1 checkbox or stable release has been created.

## Implemented

The teacher controls the STUDENT computer. Teacher generates a ten-minute
invitation; student pastes it and explicitly approves screen/keyboard/mouse.
Direct authenticated WSS is initiated from student to teacher. Ephemeral
certificate trust uses the normal TLS verifier and a random per-invitation
bearer header, with single-connection consumption. No rented host is built
in, and no public relay/credentials are invented.

Input and capture are separate student workers; decode runs off the UI.
Queues are bounded, input saturation fails closed, pending video is latest
frame only, and at most three frames await cumulative viewer ACKs. A stalled
viewer times out. Capture/encoding, network feedback and hysteresis choose
a 60/30/15/5 FPS target; these are targets, not measured delivery rates.
JPEG quality is 90 and aspect-preserving dimensions are bounded to 1080p.
Unchanged captured images are not repeatedly encoded/transmitted.

STOP/deny/drop revoke local flags synchronously and clear pending work.
A native input gate serializes short input operations against revocation.
Native input RAII cleanup releases held keys/buttons on normal/error exits;
worker panics revoke the session. Physical OS release still requires testing.

The teacher viewer letterboxes video, maps pointer coordinates, supports
local minimize/disconnect, and releases remote inputs when focus is lost.
The native English UI states that applications/actions run on the STUDENT.

Mac student permission checks request Screen Recording, Accessibility and
Input Monitoring in that order, only after the explicit ALLOW action. Missing
grants fail closed, with user-controlled System Settings/restart guidance.
No global event listener or key recorder was added for the third grant.
Mac bundles retain ad-hoc signing and now carry source/license information.

RustDesk's bounded RTT estimator is reused from the immutable revision in
`third_party/rustdesk/NOTICE.md`, with the full AGPL text and notices. **This
is initial QoS reuse, not integration of RustDesk's capture/hardware codecs.**
That engine work remains required by the product's original architecture.

## Local evidence on Windows x64

Rust 1.99.1, MSVC 14.44.35207, Windows SDK 10.0.26100.0:

- `cargo fmt --all -- --check`: passed.
- `cargo check --locked --workspace --all-targets`: passed.
- `cargo test --locked --workspace`: 36 tests passed, zero failed:
  2 transport queue unit tests, 6 real loopback WSS tests, 18 desktop/policy
  tests, 7 existing session-core and 3 existing relay tests.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `python -m unittest discover -s tests -v`: 3 passed; Bash syntax test
  skipped on this Windows host because Bash is unavailable.
- `cargo build --locked --release -p yt-remote-desktop`: passed.
- Native Windows GUI launch, home/role text and license/source page were
  visually inspected. The standard native Close control closed the window.
  Alt+F4/coordinate close attempts did not establish closure; only the native
  accessibility Close action did. Full viewer minimize/focus is untested.

The WSS integration tests use only 127.0.0.1 with synthetic payloads: consent
gating and direction, STOP, denial/disconnect, replay, expiry/wrong token/
replaced certificate, invalid/oversized work, ordered key edges and the
bounded viewer ACK window. They capture no desktop and inject no OS input.
The flow-control regression was observed failing before its implementation;
new code-paste/formula-key tests also exposed failures before their fixes.

## Limits, review and next work

No router/firewall setting was changed and no public listener or real
sharing was started during these development tests. At runtime, explicit
teacher generation attempts a leased UPnP mapping; discovery/removal depends
on gateway behavior and uses the library's bounded HTTP timeouts. No permanent
lease fallback or automatic firewall modification exists. Mapping cleanup is
best effort, with lease expiration as backup; its real behavior is untested.
This adapter currently discovers IPv4 UPnP only, with no IPv6 discovery or
relay fallback. It therefore cannot promise connectivity on every modem/ISP.

Independent helper tasks hit the account usage limit before completing
their reports/reviews. The primary implementer completed integration and
self-review with tests; a fresh independent security/code review remains
required. Keep the PR draft until that and native CI are assessed.

Physical Windows/Mac installation, OS permission prompts, Retina/scaling,
all keyboard layouts, true fullscreen/minimize, STOP with OS inputs held,
real different-city connectivity and performance benchmarks were NOT run.
No measured 1080p60 or improvement over upstream RustDesk/NoMachine is claimed.
See `docs/NOMACHINE-QUALITY-GATES.md` for the remaining engine/benchmark gates.
Subsequent native CI results belong in the PR body for the exact commit;
do not create a new CI-triggering commit merely to record its own result.
