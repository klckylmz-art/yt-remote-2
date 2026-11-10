# YT Remote 3.0 — RustDesk-first product contract

Status: development / unverified. Base: RustDesk upstream (AGPL-3.0);
the upstream engine has **not yet been integrated**. This version is not a
release, nor a claim of improved performance.

## Native-only experience

- Windows 10/11 x64 executable/installer, macOS Intel and Apple Silicon app/dmg.
- No Google sign-in, registration, email, password, subscription or user account.
- Student: GENERATE CODE (STUDENT) → COPY CODE → explicit per-request
  ALLOW SCREEN & KEYBOARD or DENY → STOP SHARING NOW.
- Teacher: paste code → CONNECT → fullscreen borderless viewer with
  small top-right minimize and disconnect.
- One 16-character code grouped XXXX-XXXX-XXXX-XXXX; app handles all
  signaling addresses. No user-facing IP, port or URL.
- Screen, mouse and keyboard ONLY. No audio, camera, files, clipboard sync
  or unattended access.
- OS screen-recording/accessibility permission remains user controlled.

## Architecture and security

Use a pinned, license-reviewed RustDesk revision for capture, encoding,
decoding, input and transport where technically and legally suitable.
Do not bypass upstream OS security controls. The YT consent gateway must
authorize each teacher request before **any** frame or input is forwarded.
DENY, CANCEL, STOP, expiry and network disconnect revoke tokens and release
pressed input. Old tokens must never authorize reconnection. No hardcoded
public hostname, credential or insecure TLS fallback.

The RustDesk engine integration is a *replacement direction*, not evidence
that existing YT 2.0 transport is production-ready. Maintain old code until
an integrated replacement passes regression tests.

## Measurable improvement targets

Benchmark unmodified RustDesk and YT 3.0 on the same devices/network:
1080p 60 FPS where supported, adaptive 30 FPS on congestion, capture-to-display
latency percentiles, bandwidth, CPU, reconnection, and Excel formula legibility.
Publish measured numbers only; prefer latest-frame-wins queues and bounded
memory over unbounded buffered latency.

## Gates before distribution

1. Pin upstream revision; audit AGPL-3.0 obligations, third-party licenses
   and disabled feature paths; provide corresponding source as required.
2. Implement and test consent gateway and upstream adapter; negative tests
   prove no frames/input without approval or after revocation.
3. Verify Windows x64/macOS Intel/macOS ARM CI and real native installations.
4. Verify actual TLS relay across separate networks and STOP during transmission.
5. Benchmark and provide SHA256 for genuine artifacts.
6. Publish stable release only after all checks and physical tests pass.

See `docs/rustdesk-upstream.json` for machine-readable adoption gates.
