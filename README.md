# YT Remote 2.0

A new Rust-based, consent-first remote-desktop project. **This is a development starter, not a working remote-desktop release.**

## Required user flow

1. The student clicks **GENERATE CODE (STUDENT)**.
2. The student copies one 16-character session code and sends it to the teacher.
3. The teacher pastes that code and clicks **CONNECT**.
4. The student receives a request and must explicitly click **ALLOW SCREEN & KEYBOARD**.
5. Only an approved, active session may start screen streaming and mouse/keyboard control.
6. The student can click **STOP SHARING NOW** at any time.

All user-facing copy must be in English. No IP address, port or HTTPS invitation link input; no audio, camera, file transfer or clipboard sync.

## Starter scope

- `crates/session-core`: code generation and explicit-consent lifecycle.
- `crates/signaling-server`: localhost-only proof-of-concept authorization API.
- `.github/workflows/ci.yml`: Rust checks for Linux, Windows and macOS.

**Not implemented:** graphical desktop UI, native screen capture, hardware codec, P2P/relay, remote controls, app installers and actual device tests. No public deployment. Do not advertise this starter as finished.

## Local checks

```bash
cargo test --workspace
cargo check --workspace --all-targets
cargo run -p yt-signaling-server
```

The prototype server binds to `127.0.0.1:8080` and is unsafe to publish directly to the internet. Production deployment requires TLS, abuse protection, authenticated signaling and tested relay infrastructure.

Apple signing/notarization is not a requirement for the initial prototype, but macOS privacy grants remain mandatory and unsigned builds may require manual approval.

See `docs/PLAN.md` for milestones.
