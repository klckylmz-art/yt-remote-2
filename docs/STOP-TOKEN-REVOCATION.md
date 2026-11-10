# STOP token revocation — security regression (2026-11-10)

## Threat and change
Previously `SessionStore::end` set the session status to `Ended` but retained the teacher bearer and pending request identifier. Streaming was gated by `Approved`, yet a revoked teacher bearer could still authenticate to the status endpoint and learn the terminal session status. STOP now clears both `teacher_secret` and `request_id` while holding the session mutex, before marking the session ended. This is a defense-in-depth improvement; it is not a complete proof of zero frames in flight after STOP.

## Regression coverage
`stop_revokes_teacher_bearer_and_clears_pending_request` checks an approved teacher can stream before STOP; after STOP, the old teacher bearer fails status authentication, streaming is denied, the student sees no pending request, and the previous request cannot be approved again. Existing consent/cancel/expiry tests remain in place.

## Validation required
Run `cargo fmt --all -- --check`, `cargo test -p yt-session-core`, and `cargo check --workspace --all-targets` on a Rust-equipped runner. These commands were **not executed locally** in this change. The existing formatting gate in PR #9 is failing due to formatting differences across the workspace; do not interpret its failure as a Rust compiler failure. A green native build, real OS test, and cross-network relay proof remain separate release gates.

## Limitations
WebSocket send operations can race with STOP; this change only makes subsequent authorization checks reject the revoked bearer. Pending OS-level key/button release, bounded sends, in-flight frame guarantees, TLS/WSS deployment and physical device tests require separate work. No stable release is authorized by this change.
