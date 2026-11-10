# STOP bearer revocation regression (2026-11-10)

The main-branch session-core previously marked a session Ended but retained
the teacher bearer token and pending request identifier. Although can_stream
returned false, teacher_status still accepted the bearer. This change revokes
both fields in the same mutex-protected critical section as the Ended state.

Two regression tests cover STOP after explicit approval and STOP while a
teacher request is pending. They assert stale bearer rejection, absence of
stream authorization and removal of the pending request ID.

## Verification

Run `cargo test -p yt-session-core` and `cargo fmt --all -- --check`.
These tests were committed but **not run locally**: Rust tooling is not
available in the execution container. Check GitHub Actions before merging.
The historical run 42768943813 failed on rustfmt differences in the signaling
server; its Windows/macOS jobs were skipped. That failure is not a billing
failure or proof of a Rust compile error.

## Remaining security limitations

STOP revocation in session-core does not by itself prove that queued relay
frames, already-held keyboard/mouse buttons or in-flight WebSocket messages
are flushed. End-to-end authorization must be checked on every media/input
forwarding boundary; test those boundaries and physical devices separately.
This is not a production release.
