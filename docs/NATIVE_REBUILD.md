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
