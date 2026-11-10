# CI quality gates and evidence (2026-11-10)

This change adds automated **checks**, not a verified release. The workflow requires `cargo fmt --all -- --check`, `cargo clippy --locked -p yt-session-core -p yt-signaling-server --all-targets -- -D warnings`, and `--locked` for Rust checks, tests, and desktop release builds. `--locked` prevents silent dependency-lockfile changes in CI; it does not guarantee OS-level binary reproducibility. Clippy is initially scoped to service crates; desktop Clippy remains an open gate.

## Baseline evidence

GitHub Actions run `42607323960` (2026-11-09) reported success for Ubuntu session/relay, Windows x64 build/test/artifact, and macOS ARM64 and Intel build/test. Both macOS jobs failed in the **bundle verification** step after packaging; the macOS package artifacts were not uploaded. This is not a Rust compilation failure. Later `lipo` workflow corrections have **not** been verified by that older run.

## Required validation for this branch

1. Inspect the PR's latest GitHub Actions job conclusions and actual failing step logs. In particular, a formatting or Clippy failure must be fixed in source, not bypassed.
2. Confirm all three native OS jobs run `cargo check --locked --workspace --all-targets`, `cargo test --locked --workspace`, and `cargo build --locked --release -p yt-remote-desktop` successfully.
3. Confirm macOS architecture and archive verification and SHA256 manifest upload, and Windows binary and SHA256 manifest upload. Artifacts remain **UNTESTED** until physical installation and cross-network tests.
4. Add desktop Clippy after addressing its actual warnings; do not suppress warnings globally merely to turn CI green.

## Outstanding release blockers

No evidence here of installed/launch-tested Windows or macOS apps, a deployed public HTTPS/WSS relay, real separate-network screen/control with explicit per-session consent, revocation and held-input cleanup under failure, or measured 1080p/60→30 FPS behavior. Do not mark Issue #1 gates complete on the basis of this CI-only change.
