# P0 validation — 10 November 2026

Status: experimental native prototype; **not YT Remote 3.0 ready for release**.
Issue #1 checkboxes remain unchanged.

## Baseline and changes

Main was `1e536fb8d89cb8d3891566625a1e40a530604eef`. Reviewed open
PRs #2–#17 before editing. Continued existing PR #17 rather than opening
another CI PR. Other overlapping security, packaging and RustDesk PRs
remain separate and are not silently merged or replaced.

Inspected [run 42820308915](https://github.com/klckylmz-art/yt-remote-2/actions/runs/42820308915),
including Linux job 129843869838 logs. Linux failed at workspace rustfmt;
check/test/Clippy were skipped. Windows x64 and both macOS native jobs
passed check/test/release build and uploaded prototype packages. Those
artifacts predate this patch and do not validate this patch or installation.
The failure was not an account billing error.

Applied `cargo fmt --all` to all five Rust source files in a separate
formatting-only commit. Strict workspace Clippy then identified four
`needless_borrow` diagnostics in desktop streaming/viewer code; removed
only the redundant references in a subsequent change, with no intended
behavior change. No lint suppression or test removal was used.

Generated a real workspace Cargo.lock with Cargo and committed it. CI
uses `--locked` for compilation, tests, linting and release builds.
Rust remains on the stable channel; the toolchain itself is not pinned.
Native CI now includes strict workspace Clippy, Linux runs the existing
Python workflow tests, and push runs are limited to main so an open PR
does not also trigger a duplicate branch-push run. Native jobs remain
independent of the Linux gate, as originally proposed in #17.

Fixed #17's Python test parser: splitting at any newline followed by two
spaces returned an empty native job. It now stops at the next job key.
Observed the original test fail, then both workflow tests pass after repair.

## Local Windows evidence

Environment: Windows x64, Rust/Cargo 1.99.1, MSVC 14.44.35207 and Windows
SDK 10.0.26100.0. Rust and Microsoft Build Tools were installed locally;
no application credentials or public relay were created. The first check
attempt failed because link.exe was not yet available during installation;
subsequent checks used the completed toolchain.

Passed after the source changes:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`: 13 tests, zero failures (3 desktop, 7 session, 3 relay)
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `python -m unittest discover -s tests -v`: 2 tests, zero failures
- `cargo build --locked --release -p yt-remote-desktop`

The local EXE is a PE x86_64 binary. This is a build/architecture check,
not an installer, launch, permission, two-device, or cross-network test.
Fresh macOS validation must come from the native PR CI matrix.

## RustDesk and license review

The existing client still uses XCap/Enigo/Minifb and JPEG. **No RustDesk
engine is integrated by this CI repair.** PRs #12/#13 contain proposed
adoption documents, not an engine. Do not infer 60 FPS, hardware encoding
or performance improvements from this work.

Reviewed upstream [RustDesk's AGPL-3.0 text](https://github.com/rustdesk/rustdesk/blob/master/LICENCE)
and [Cargo manifest](https://github.com/rustdesk/rustdesk/blob/master/Cargo.toml).
Before importing or distributing a derivative, preserve copyright/license
notices, identify modifications, provide the applicable license and
corresponding source/build material with binaries (sections 4–6), and
provide the required source offer to remote users of a modified covered
program (section 13). Audit each reused component and transitive license;
the current crates' MIT metadata is not a substitute for those obligations.
Record an immutable upstream revision and review its exact source before
adoption. This initial review does not certify component compatibility.

Upstream's full application includes features outside YT Remote's allowed
scope. A narrow, consent-gated integration must exclude audio, camera,
files, clipboard synchronization and unattended access, with tests at
capture/input/network boundaries. Do not substitute upstream's stock
authorization flow for explicit approval of each student request.

## Remaining blockers and next steps

1. Inspect fresh #17 CI on this exact revision; resolve any actual platform
   errors and reconcile overlapping CI PRs before integration.
2. Review and select one existing STOP credential-revocation PR (#4/#10/#11/
   #14/#15), then verify live relay revocation and OS input release. Existing
   unit tests alone do not prove absence of in-flight frames/input.
3. Integrate pinned, audited RustDesk components using the required YT Remote
   3.0 native consent flow. Keep the prototype explicitly experimental.
4. Add live transport security tests, rate limits, resource bounds and
   responsive native I/O. Do not claim current production security.
5. The operator confirmed no server/domain exists. Public HTTPS/WSS
   deployment and actual cross-network testing therefore remain blocked;
   no relay address, DNS record, credentials or connectivity evidence is invented.
6. Physical Windows/macOS installs, OS grants, fullscreen/minimize behavior,
   restrictive NAT and equivalent-hardware RustDesk benchmarks remain
   unperformed. No stable tag/release is authorized by these build results.
