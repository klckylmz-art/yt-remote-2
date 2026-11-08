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
- [ ] Low-latency mouse/keyboard transport gated on active student approval.
- [ ] NAT traversal with deployed and tested relay.
- [ ] Revocation verified in both video and control planes.

## Phase 3 — Desktop applications
- [ ] Fully English: GENERATE CODE (STUDENT), COPY CODE, CONNECT, ALLOW SCREEN & KEYBOARD, DENY, STOP SHARING NOW.
- [ ] Teacher fullscreen view with small upper-right minimize/disconnect controls.
- [ ] Adaptive 60→30 fps target and bitrate/resolution adjustment.
- [ ] Windows x64 installer; Apple Silicon and Intel macOS .app/.dmg.
- [ ] No paid Apple signing certificate required for prototype; preserve macOS permission model.
- [ ] Real physical Windows/macOS, cross-network, restrictive-NAT and permission-denial tests.
- [ ] Publish only verified releases with checksums and test results.

No microphone, audio, camera, file transfer or clipboard sync. IP addresses, ports and invitation links must never be requested from users.
