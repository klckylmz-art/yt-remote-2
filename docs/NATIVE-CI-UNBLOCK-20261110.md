# Native CI unblock — 2026-11-10

Baseline: run 42768943813 failed in the Linux session-and-relay job on
`cargo fmt --all -- --check` (rustfmt diff). Its dependent native job was
skipped, so that run **does not establish** a Windows or macOS compilation
result. This was a formatting failure, not evidence of a GitHub billing block.

This change removes `desktop-build.needs: session-and-relay` so that the
Windows x64, macOS ARM and macOS Intel builds run independently of Linux
failures. It also explicitly checks formatting and Clippy on Linux.
The workflow may remain red until the existing formatting differences are
resolved; do not remove tests to make it green.

Regression guard: `python3 -m unittest discover -s tests -p 'test_native_ci_independence.py'`.
It checks workflow structure only; it is **not** proof that native binaries
compile or install. Review fresh GitHub Actions logs, then fix each actual
platform error and verify artifacts before publishing a release.
