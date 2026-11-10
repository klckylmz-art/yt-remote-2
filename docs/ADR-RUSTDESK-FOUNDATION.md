# ADR: Evaluate RustDesk as the YT Remote 2.0 transport foundation

Status: **Proposed — not integrated**. This decision does not certify RustDesk as faster or establish a working release.

## Decision

Prioritize evaluating RustDesk's maintained native capture, input, codec and
transport implementations rather than reimplementing those systems without
benchmarks. Retain YT Remote's single-code, per-session explicit consent UI.
Do not copy upstream code until the licensing and security review is complete.

## Required adoption procedure

1. Review upstream repository and exact commit, record the full 40-character
   revision in `docs/rustdesk-upstream.json`; assess compatibility of
   AGPL-3.0 obligations with the intended distribution, and retain notices
   and corresponding-source availability. Do not assume individual crates
   share the repository's license: verify each component and its dependencies.
2. Audit upstream features and disable or exclude audio, camera, file transfer,
   clipboard sync and unattended access. Local COPY CODE is allowed.
3. Build a narrow adapter for screen frames, input and connection events.
   Consent must be enforced at the boundary **before** any frame or input,
   not solely by hiding GUI controls. STOP/DENY/CANCEL/disconnect must revoke
   authorization and release pressed keys/buttons.
4. Benchmark against an unmodified pinned RustDesk revision on the same
   Windows/macOS hardware and network: measured capture-to-display latency,
   FPS (60/30 target), CPU, bandwidth, frame loss, Excel text legibility and
   reconnection behavior. Never claim faster without measured evidence.
5. Test Windows x64, macOS Intel/ARM, real cross-network TLS and physical OS
   permission flows. Preserve the release gates from Issue #1.

The manifest and its Python unittest are an integration policy guard, **not**
a RustDesk dependency or performance improvement. Run:

```sh
python3 -m unittest discover -s tests -p 'test_rustdesk_policy.py'
```

Open blockers: upstream component/license review, pinned revision, native
integration, public relay, physical-device and performance tests.
