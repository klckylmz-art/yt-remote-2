# YT Remote 2.0 — implementation milestones

## Phase 1 — Consent-first session service
- [x] Student-created one-piece code, separate student and teacher secrets.
- [x] Request, allow, deny, expire, and revoke state transitions.
- [x] Unit tests for explicit permission and rejection of replays.
- [x] Local-only HTTP proof-of-concept authorization API.
- [ ] External authentication, throttling, TLS reverse proxy, encrypted secret storage, request origin authentication and persistent session cleanup.
- [ ] Authenticated real-time signaling without polling.

## Phase 2 — Native transport
- [ ] Review RustDesk integration/licensing or choose an independently maintained engine.
- [ ] Windows Graphics Capture and macOS ScreenCaptureKit.
- [ ] Native hardware encoding, low-latency decoding, clear Excel text.
- [x] First authenticated, consent-gated WebSocket relay for JPEG frames and allowlisted input commands (source only; builds not verified).
- [x] Experimental native desktop capture/input and teacher viewer in Rust (terminal launcher; builds not verified).
- [ ] Production-grade low-latency mouse/keyboard transport and hardware-encoded video.
- [x] Optional Caddy TLS + Docker deployment template (not deployed).
- [ ] Real deployed public relay, abuse prevention and restrictive-NAT tests.
- [x] Source unit test checks relay access before approval, after approval and after student stop.
- [ ] Live integration test proving revocation of an ongoing real video/control stream.

## Phase 3 — Desktop applications
- [x] First English graphical launcher includes GENERATE CODE (STUDENT), COPY CODE, CONNECT, ALLOW SCREEN & KEYBOARD, DENY, STOP SHARING NOW (source only, device behavior not verified).
- [ ] Professional polish, accessibility, device-tested UI and native minimize control.
- [x] Borderless teacher viewer scaled to primary display and top-right disconnect target (source only, device behavior not verified).
- [ ] Native minimize control and true OS fullscreen behavior tested across Windows/macOS.
- [ ] Adaptive 60→30 fps target and bitrate/resolution adjustment.
- [x] Windows release-mode .exe build and macOS Intel/Apple Silicon .app/.dmg packaging configured in CI (new builds not verified).
- [ ] Verified Windows installer and signed/notarization-free Mac installation on real devices.
- [ ] No paid Apple signing certificate required for prototype; preserve macOS permission model.
- [ ] Real physical Windows/macOS, cross-network, restrictive-NAT and permission-denial tests.
- [ ] Publish only verified releases with checksums and test results.

No microphone, audio, camera, file transfer or clipboard sync. IP addresses, ports and invitation links must never be requested from users.

## Relay socket-exit revocation (2026-11-09; unverified CI)
- On a student WebSocket exit, the relay now ends the student code and invalidates any teacher token. On a teacher WebSocket exit, it invalidates that teacher token and ends the approved session. A consent gate failure also follows the same cleanup path.
- Added a Rust unit regression for both roles' socket-exit revocation; the test exercises the revocation helper and session store, **not** an actual network socket or operating-system key release. Rust CI and physical-device results for this change are not yet verified.
- Remaining security gap: socket close is not instantaneous proof of a lost peer. Until heartbeat/lease enforcement, in-flight frame/input race tests, and native held-key release on disconnect are verified, STOP/network-drop safety is not release-ready.

## macOS architecture verification fix (2026-11-09; pending rerun)
- Examined Actions run `42607323960`: Ubuntu session/relay and Windows x64 jobs succeeded; both macOS jobs compiled, tested, built, and packaged successfully, but failed at the `Verify macOS bundle and create SHA256 manifest` step. This is not evidence of a Rust compile failure or a billing-related runner startup failure.
- Corrected `lipo` invocation to `lipo -verify_arch "$arch" <bundle executable>` so the tool receives the architecture before the binary. The existing CI step now acts as an architecture regression gate for both `arm64` and `x86_64`, followed by nonempty ZIP/DMG checks and SHA256 generation.
- **Not yet verified:** CI rerun of this change, artifact upload, physical launch/install, screen permissions, or cross-network operation. Do not mark Stage 0 or release gates complete until new run logs confirm them.

## Concurrent STOP versus ALLOW regression (2026-11-09; pending CI)
- Added `concurrent_student_stop_and_teacher_approval_cannot_resurrect_session` to session-core. A barrier starts ALLOW and STOP on separate threads, accepts either valid ordering, then asserts the teacher bearer is revoked, student status is terminal, and the code cannot accept another request.
- This proves a session-store invariant only if the new test passes in CI; it does **not** establish zero in-flight network frames, native key/button release, or physical cross-network behavior. Keep those release gates open.

## Session TTL hardening (2026-11-10; CI not yet verified)
- The session store now caps every requested lifetime to 900 seconds, including direct library callers. Previously `Duration::MAX` could overflow `Instant` arithmetic or produce an unexpectedly long authorization window.
- Added `oversized_session_ttl_is_capped_without_panicking` regression: it creates a session with `Duration::MAX`, checks that approval is still mandatory, and checks STOP revocation. This does not wait 15 minutes to verify the upper bound; expiry integration tests and live socket checks remain open.
- Evidence: source commit and test are recorded on the native rebuild branch. No new CI result, cross-network test, or physical-device test is claimed by this entry.

## Session TTL compile blocker correction (2026-11-10; CI pending)
- Source inspection found literal `\\n` escape sequences inside the `Entry` struct initializer after `status: Status::Waiting,`. These are not Rust line breaks and can cause parsing/compilation failure on the native rebuild branch.
- Replaced the literal escape sequences with actual source newlines, preserving the 900-second TTL cap and the existing `oversized_session_ttl_is_capped_without_panicking` regression test.
- Evidence is the committed source diff; the older Actions run `42607323960` predates this change and **cannot** verify it. Required next check: `cargo check --locked -p yt-session-core --all-targets` and `cargo test --locked -p yt-session-core` in a new CI run, then Windows/macOS matrix and artifact verification. No physical device or public relay result is claimed.
