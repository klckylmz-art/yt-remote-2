# YT Remote 3.0 — experimental native direct mode

**Not a verified release. No NoMachine-equivalence or measured FPS claim.**

The teacher controls the **STUDENT computer**. Applications, mouse actions,
keyboard input and edits run on the student computer. Different cities and
modems are the intended test scenario; actual separate-network proof is pending.

## Direct mode (new default native interface)

1. Teacher clicks **GENERATE CODE (TEACHER)**, then **COPY CODE** and sends the
   complete invitation privately to the student. The invitation lasts ten minutes.
2. Student clicks **PASTE CODE**, then **CONNECT**.
3. Student explicitly clicks **ALLOW SCREEN & KEYBOARD** or **DENY**. No screen
   is captured/transmitted and no input is injected before permission.
4. Teacher sees the STUDENT screen and controls it. Student can use
   **STOP SHARING NOW**. Teacher has local minimize/disconnect controls; Escape
   also disconnects. Ended sessions require a fresh invitation and approval.

No accounts, passwords, subscriptions, audio, camera, files or clipboard
synchronization. Local copying/pasting of the invitation is permitted.

The direct invitation is longer than the old 16-character relay lookup code:
it contains the endpoint, ephemeral TLS certificate and random session
capability. TLS uses that certificate as its trust anchor with standard
certificate/name validation. No accept-all verifier is used. Keep the code private.

The teacher's app attempts a ten-minute UPnP router mapping and removes it
when the session worker exits; no permanent mapping fallback or automatic
firewall rule is added. This needs a reachable teacher-side public IPv4 and
supported router/ISP/firewall behavior. No rented relay or domain is contacted
by the direct session. IPv6 direct discovery and relay fallback are not yet
implemented in this adapter. Unsupported networks report a failure; they do
not silently disable encryption or ask students to configure an IP address.

The development tests use loopback only and do not touch router mappings,
OS permission grants, the desktop screen or native mouse/keyboard input.

## Build and verification

Install Rust plus native compiler tools (MSVC/Windows SDK on Windows;
Apple command-line tools on macOS). Then:

```sh
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --release -p yt-remote-desktop
cargo run --locked -p yt-remote-desktop
```

On matching macOS architecture, package a clean committed checkout with
`bash packaging/macos/package.sh macOS-Intel` or `macOS-Apple-Silicon`.
See [Mac permissions](docs/MACOS-PERMISSIONS.md): student-side checks request
Screen Recording, Accessibility, then Input Monitoring, in that order, only
following explicit student approval. Missing grants fail closed. macOS may
require app restart; grants remain controlled by the device owner.

The new video adapter uses adaptive target FPS, JPEG quality 90, 1080p bounds,
unchanged-frame suppression, bounded latest-frame mailboxes and viewer ACKs.
This is not a hardware codec engine. RustDesk capture/codec integration and
[NoMachine quality comparison](docs/NOMACHINE-QUALITY-GATES.md) remain required.
Physical installs, true fullscreen/Retina/minimize behavior, keyboard layouts,
OS input release and real different-network tests remain unverified.

The desktop derivative includes AGPL RustDesk QoS code. See [COPYING](COPYING),
[third-party notices](THIRD_PARTY_NOTICES.md) and the exact upstream revision
in [component provenance](third_party/rustdesk/NOTICE.md). CI distributions
include the matching source pointer and remain **UNTESTED**. No stable release
or Issue #1 completion follows merely from successful compilation.

## Historical relay prototype reference

The following documentation describes the older relay prototype. Its native
launcher is now developer-only via `cargo run -p yt-remote-desktop -- --relay`;
its localhost/server configuration is not the new direct-mode workflow.

# YT Remote 2.0

**Development status: native screen/control prototype committed. NOT a verified release.**

The finished product must provide a fully English desktop UI: student clicks **GENERATE CODE (STUDENT)** and **COPY CODE**, teacher pastes the code and clicks **CONNECT**, and the student must click **ALLOW SCREEN & KEYBOARD** before any screen or input access begins. No user enters an IP address, port or link. The final UI requires a fullscreen teacher viewer with compact minimize/disconnect buttons, Windows .exe, and Intel/Apple Silicon macOS .app/.dmg.

## Currently implemented in source

- `crates/session-core` — 16-character private code generation, distinct student/teacher tokens, permission request, explicit allow/deny, expiry, and session revocation. Unit tests cover consent.
- `crates/signaling-server` — localhost HTTP authorization API and a WebSocket relay for authenticated JPEG frames (student → teacher) and allowlisted mouse/keyboard JSON commands (teacher → student). Frames are never accepted by the relay before an approved session. No file, audio, camera, or clipboard channels.
- `crates/desktop` — **experimental native client** with an English graphical launcher, code generation/copying, teacher code entry, student ALLOW/DENY permission screen, and STOP SHARING NOW. It uses XCap desktop capture, Enigo input and a basic Minifb teacher viewer. Developer CLI commands remain available. Uses JPEG frames, about 15 FPS target. It does **not** implement hardware H.264/AV1, 60 FPS, adaptive 30 FPS, or professional fullscreen controls.
- `.github/workflows/ci.yml` — checks/tests and release-mode native prototype builds on Windows x64, macOS Apple Silicon, and macOS Intel. Artifacts are explicitly marked UNTESTED. Windows builds provide a prototype .exe; macOS jobs attempt an ad-hoc signed .app.zip and .dmg (not notarized). These have not been installed or tested on physical devices and are not production installers.

## Local prototype usage (trusted machines only)

The server defaults to `127.0.0.1:8080`, so use the same machine or a **trusted, separately configured TLS reverse proxy** for remote tests. The server is *not* production hardened. Do not expose its HTTP port directly to the internet.

```bash
cargo test -p yt-session-core -p yt-signaling-server
cargo run -p yt-signaling-server

# Launch the new graphical student/teacher interface:
cargo run -p yt-remote-desktop

# Optional developer-only text modes:
cargo run -p yt-remote-desktop -- student
cargo run -p yt-remote-desktop -- teacher XXXX-XXXX-XXXX-XXXX
```

The graphical student receives an ALLOW/DENY prompt and may click STOP SHARING NOW. Graphical teacher code entry supports typing or pasting the entire code. Developer CLI student mode still uses the terminal. The teacher's provisional viewer closes with Escape. Remote streaming requires valid macOS Screen Recording and Accessibility grants. For a TLS-proxied server, set `YT_REMOTE_SERVER=https://your-server` on each machine **during development**. This variable is internal deployment configuration, not something the final app should ask users to enter.

## What must happen before a real release

1. CI must actually pass, and builds must be downloaded and installed on physical Windows, Apple Silicon and Intel Macs.
2. Finish and polish the new graphical launcher, implement responsive fullscreen teacher controls, minimize/disconnect and a robust state machine; validate with real device UI tests.
3. Replace JPEG-frame prototype with hardware-accelerated capture/encoding and low-latency rendering, with measured 60/30 FPS adaptation and Excel text clarity.
4. Deploy and security-harden a reachable TLS signaling/relay service; test restrictive NAT and network failures.
5. Test denial/no-frame/no-input, revocation during streaming, reconnect, different networks, full screen, minimize and exit.
6. Package Windows .exe and macOS .app/.dmg. Paid Apple Developer certificates are not a requirement for prototypes; unsigned apps may need local user approval.

**Do not describe source changes, CI configuration, or unverified artifacts as a working release.**

See [Implementation plan](docs/PLAN.md).
