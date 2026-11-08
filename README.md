# YT Remote 2.0

**Development status: native screen/control prototype committed. NOT a verified release.**

The finished product must provide a fully English desktop UI: student clicks **GENERATE CODE (STUDENT)** and **COPY CODE**, teacher pastes the code and clicks **CONNECT**, and the student must click **ALLOW SCREEN & KEYBOARD** before any screen or input access begins. No user enters an IP address, port or link. The final UI requires a fullscreen teacher viewer with compact minimize/disconnect buttons, Windows .exe, and Intel/Apple Silicon macOS .app/.dmg.

## Currently implemented in source

- `crates/session-core` — 16-character private code generation, distinct student/teacher tokens, permission request, explicit allow/deny, expiry, and session revocation. Unit tests cover consent.
- `crates/signaling-server` — localhost HTTP authorization API and a WebSocket relay for authenticated JPEG frames (student → teacher) and allowlisted mouse/keyboard JSON commands (teacher → student). Frames are never accepted by the relay before an approved session. No file, audio, camera, or clipboard channels.
- `crates/desktop` — **experimental command-line native client**, using XCap desktop capture, Enigo input, and a Minifb teacher viewer. It is a basic integration step, not the final graphical desktop client. Uses JPEG frames, about 15 FPS target. It does **not** implement hardware H.264/AV1, 60 FPS, adaptive 30 FPS, or professional fullscreen UI.
- `.github/workflows/ci.yml` — checks/tests and release-mode native prototype builds on Windows x64, macOS Apple Silicon, and macOS Intel. Artifacts are explicitly marked UNTESTED and are not production installers.

## Local prototype usage (trusted machines only)

The server defaults to `127.0.0.1:8080`, so use the same machine or a **trusted, separately configured TLS reverse proxy** for remote tests. The server is *not* production hardened. Do not expose its HTTP port directly to the internet.

```bash
cargo test -p yt-session-core -p yt-signaling-server
cargo run -p yt-signaling-server

# Student's computer (separate terminal)
cargo run -p yt-remote-desktop -- student

# Teacher's computer, paste the student's one-piece code
cargo run -p yt-remote-desktop -- teacher XXXX-XXXX-XXXX-XXXX
```

The student receives a text request and types `ALLOW`; the student can press Enter during sharing to revoke permission. The teacher's viewer closes with Escape. Remote streaming requires valid macOS Screen Recording and Accessibility grants. For a TLS-proxied server, set `YT_REMOTE_SERVER=https://your-server` on each machine **during development**. This variable is internal deployment configuration, not something the final app should ask users to enter.

## What must happen before a real release

1. CI must actually pass, and builds must be downloaded and installed on physical Windows, Apple Silicon and Intel Macs.
2. Replace the text-terminal student approval with the requested native graphical desktop UI.
3. Replace JPEG-frame prototype with hardware-accelerated capture/encoding and low-latency rendering, with measured 60/30 FPS adaptation and Excel text clarity.
4. Deploy and security-harden a reachable TLS signaling/relay service; test restrictive NAT and network failures.
5. Test denial/no-frame/no-input, revocation during streaming, reconnect, different networks, full screen, minimize and exit.
6. Package Windows .exe and macOS .app/.dmg. Paid Apple Developer certificates are not a requirement for prototypes; unsigned apps may need local user approval.

**Do not describe source changes, CI configuration, or unverified artifacts as a working release.**

See [Implementation plan](docs/PLAN.md).
