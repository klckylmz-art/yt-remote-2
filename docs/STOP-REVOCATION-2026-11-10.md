# STOP bearer revocation regression — 2026-11-10

The `SessionStore::end` transition now clears `teacher_secret` and
`request_id` under the same mutex that changes status to `Ended`.
Before this change, `can_stream` rejected ended sessions but
`teacher_status` still authenticated the stale teacher bearer.

New unit tests in `crates/session-core/src/lib.rs` cover approved and
pending sessions: STOP invalidates the teacher token, removes the request
identifier, rejects request replay, and forbids subsequent streaming.

## Verification

Source inspection confirms the change and the added assertions. The
regression tests have **not** been executed in this environment. Run:

```sh
cargo fmt --all
cargo fmt --all -- --check
cargo test -p yt-session-core
cargo test --workspace
cargo check --workspace --all-targets
```

The prior workflow run 42768943813 failed its rustfmt check, so its
Windows/macOS jobs were skipped. This is not evidence of a billing or
compiler failure. A new CI run must pass before merging.

## Limitations

This is authorization-state hardening, **not** proof that already queued
frames or OS-level held keys/buttons are flushed. That requires transport
and native input integration tests. RustDesk integration, real TLS relay,
physical OS tests, performance benchmarks and production packaging remain
unverified. No stable release should be created from this patch alone.
