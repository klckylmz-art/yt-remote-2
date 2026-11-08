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
- [ ] Fully English: GENERATE CODE (STUDENT), COPY CODE, CONNECT, ALLOW SCREEN & KEYBOARD, DENY, STOP SHARING NOW.
- [ ] Teacher fullscreen view with small upper-right minimize/disconnect controls.
- [ ] Adaptive 60→30 fps target and bitrate/resolution adjustment.
- [ ] Windows x64 installer; Apple Silicon and Intel macOS .app/.dmg.
- [ ] No paid Apple signing certificate required for prototype; preserve macOS permission model.
- [ ] Real physical Windows/macOS, cross-network, restrictive-NAT and permission-denial tests.
- [ ] Publish only verified releases with checksums and test results.

No microphone, audio, camera, file transfer or clipboard sync. IP addresses, ports and invitation links must never be requested from users.
