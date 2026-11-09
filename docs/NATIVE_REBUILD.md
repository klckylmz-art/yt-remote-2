# Consent-first native rebuild (proposal, not a release)

This branch preserves the existing main branch and begins with a security regression fix. Do not delete the prototype until a replacement passes equivalent tests. Windows x64 and macOS x86_64/arm64 only; no web/mobile client.

## Engine decision gate

Evaluate RustDesk's maintained native capture, codec, input, and networking components against the current implementation. Reuse only components whose dependencies, build portability, security, and AGPL-3.0 obligations have been reviewed. Do not copy RustDesk code into the MIT-licensed workspace without an explicit licensing decision. A RustDesk-inspired architecture is not proof that its code has been integrated.

## Required connection contract

Student creates a single XXXX-XXXX-XXXX-XXXX code; teacher submits it; the student must approve that specific request before any frames or input flow. Deny, cancel, expiration, disconnect and STOP SHARING NOW must revoke the corresponding credentials and clear pending input. Every new connection requires fresh approval. The application manages server configuration; neither user enters an IP, URL or port. Only screen, keyboard and mouse may be transmitted.

## Delivery order and evidence

1. Recover green reproducible CI and Cargo.lock; classify runner/billing failures separately from compiler errors.
2. Prove consent lifecycle and zero unauthorized frames/input with unit and live integration tests.
3. Validate native Windows/macOS capture/input and OS permissions on physical machines.
4. Integrate a reviewed low-latency engine and measured adaptive 60/30 FPS, with P2P and authenticated relay fallback.
5. Build and install real .exe/.app/.dmg artifacts, verify SHA256 and cross-network tests before a stable release.

This document and the accompanying test do not certify any OS build, RustDesk integration, public relay, or working installer.

## Current security regression coverage (source changes; CI pending)

The session store now clears the teacher bearer token and pending request identifier on both student STOP and student DENY. The denial regression asserts the old teacher credential returns `Unauthorized` and the student's pending request ID is absent. This does not by itself prove that a previously established WebSocket stops immediately: a live relay test is still required, including held-key release and in-flight frame cancellation.

## Teacher disconnect API (source change; unverified)

`POST /v1/sessions/{code}/disconnect` requires the current teacher bearer credential. For a pending request it clears the request and returns the student to Waiting; for an approved request it revokes the teacher token and ends the student session. Reusing the old token must return Unauthorized. The new unit test covers pending and approved disconnects, invalid bearer credentials, replay rejection and fresh approval after a canceled pending request. This is **not** yet wired to every native viewer exit path, and does not guarantee in-flight WebSocket frames or held input are flushed. These remain mandatory integration/device tests.

CI job logs were not retrievable through the available commit-run lookup (no PR-triggered runs returned); no Windows/macOS build or local Rust test success is asserted. Do not treat missing CI evidence as a passing check or as a compiler failure.

## Verified CI failure diagnosis (run 42607323960, 2026-11-09)

- Ubuntu session-and-relay: success (check and tests).
- Windows x64: success (workspace check, tests, release build, SHA256 manifest, prototype artifact upload).
- macOS ARM64 and Intel: workspace check, tests, release build and ad-hoc app/DMG packaging succeeded; both failed in the subsequent architecture-verification step.
- Actual error: `lipo: unknown architecture specification flag: dist/YTRemote.app/Contents/MacOS/YTRemote in specifying -verify_arch operation`. The workflow incorrectly supplied `-verify_arch` before the input file. Correct invocation is `lipo <binary> -verify_arch <arch>`.
- The workflow change fixes this observed CI command error only. Its own CI run, generated checksums and uploads must be verified before claiming success. These CI jobs are not physical-device or cross-network tests.

## Viewer disconnect hardening — 2026-11-09

The teacher's WebSocket worker now attempts the authenticated `POST /v1/sessions/{code}/disconnect` when its viewer closes, the socket closes, the connection drops, or the handshake fails. This avoids leaving an approved teacher token valid after a normal window exit. A session-core regression test verifies that an approved teacher disconnect invalidates its token and requires a newly generated student code and a fresh approval.

**Limits:** HTTP disconnect is best-effort and can fail when the network is down; server-side expiry and authorization checks remain mandatory. The current network worker can block during HTTP calls and this change does not yet guarantee that every in-flight input is released on the student's OS. The server must still enforce per-message consent and the student must release held inputs when its own socket closes. No physical device or cross-network validation has been performed for this change.

## macOS packaging verification regression (2026-11-09)

The workflow previously invoked `lipo <binary> -verify_arch <arch>`, which places the option after the file. Correct invocation is `lipo -verify_arch <arch> <binary>`. The CI workflow now also asserts that both ZIP and DMG are nonempty **before** calculating SHA256. These are packaging checks, not physical installation tests.

Verification checklist on both `macos-15` and `macos-15-intel` GitHub-hosted runners:

1. Confirm `rustc -vV` host matches expected ARM64 / x86_64 target.
2. Confirm `cargo check --locked --workspace --all-targets`, `cargo test --locked --workspace`, and release build pass.
3. Confirm `lipo -verify_arch` succeeds and both archives exist with nonzero size.
4. Confirm SHA256 manifests and `UNTESTED` artifacts are uploaded.
5. Keep clean-machine install, screen permission, live relay and two-device cross-network validation open until evidence exists.

The updated workflow has **not yet been observed passing**; a successful GitHub commit is not CI proof. Do not mark Issue #1 complete based on this change alone.
