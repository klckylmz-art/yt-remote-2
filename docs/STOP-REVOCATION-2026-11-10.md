# STOP bearer revocation — 2026-11-10

## Observed baseline
The main-branch `SessionStore::end` set `Status::Ended` but retained `teacher_secret` and `request_id`. `can_stream` already denied ended sessions, but `teacher_status` still accepted the old bearer and returned `Ended`, and `student_status` still exposed the stale pending request ID. This is a credential lifecycle defect.

The prior CI run 42768943813 failed at `cargo fmt --all -- --check` on the session-and-relay job; the desktop build was skipped. This is a formatting gate failure, not a billing failure or a Rust compile result.

## Change
STOP now clears the teacher bearer and pending request ID under the existing mutex before setting Ended. Two unit tests cover approved and unapproved pending requests, authentication after STOP, denial of streaming, stale approval replay, and inability to request on an ended code.

## Verification and limitations
Source change and test assertions were reviewed, but this environment cannot run the repository's Rust toolchain or native Windows/macOS runners. **These new tests are not yet confirmed passing.** Required checks on the PR: `cargo fmt --all -- --check`, `cargo test -p yt-session-core`, `cargo check --workspace --all-targets`, and Windows/macOS builds. Existing formatting debt must be corrected separately; do not disable the formatting gate.

Revoking the server-side bearer does **not** by itself prove that queued WS frames or locally held input keys/buttons are released. That requires a live relay/injection integration test and platform-specific device testing. No cross-network test or install test was performed. Do not check off Issue #1 release gates.
