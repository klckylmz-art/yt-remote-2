# CI native matrix independence — 2026-11-10

## Observed baseline

GitHub Actions run 42768943813, job 129671950757 failed at
`cargo fmt --all -- --check` (rustfmt produced diffs). The native build job
was marked **skipped**, not failed; no Windows/macOS compile result can be
inferred from that run. Do not mislabel this as a billing restriction.

## Change

The Windows x64, macOS arm64 and macOS x86_64 build matrix no longer depends
on the Ubuntu service test job. This means genuine platform-specific failures
are visible even when Linux formatting/security checks fail. Both tracks
still fail the overall workflow when they fail. Formatting is explicitly
checked; Ubuntu also runs Clippy with warnings denied. Native builds keep
their architecture checks, release build, packaging, SHA256 and artifact
presence assertions.

## Regression check

`python3 -m unittest discover -s tests -p test_ci_native_matrix.py`

This tests the workflow contract only. It does **not** prove any native build
passes. Review new GitHub Actions run logs and artifacts before reporting
Windows/macOS success. Physical installation and cross-network tests remain
separate release gates. Avoid deploying or purchasing infrastructure.
