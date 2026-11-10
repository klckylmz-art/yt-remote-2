# Prototype artifact checksum verification (2026-11-10)

## Scope
This change adds fail-closed checks **before upload** of the existing Windows x64 executable and macOS Intel/Apple Silicon ZIP/DMG prototypes. It does not add a release, installer, signing certificate, external host, or new user-facing feature.

## Prior evidence and current blockers
GitHub Actions run [42607323960](https://github.com/klckylmz-art/yt-remote-2/actions/runs/42607323960) completed the Ubuntu service job and Windows x64 job successfully. Both macOS jobs compiled, tested and packaged successfully but failed at the bundle verification step; they did not upload macOS artifacts. The `lipo` argument-order change on `main` has **not** been verified by a newer successful run.

The repository's default branch currently has **no root `Cargo.lock`** (GitHub contents API returned 404 on 2026-11-10). Therefore, do **not** merge another PR that enables `cargo ... --locked` until a real Cargo-generated lockfile has been committed and checked. A fabricated lockfile is unsafe. In particular, PR #5's locked commands require this prerequisite.

## Checks added
- Windows: after creating `SHA256SUMS.txt`, assert the manifest entry has the expected executable name/hash and rehash the exact executable. Any mismatch throws and blocks upload.
- macOS ARM64/Intel: run `shasum -a 256 -c` on the generated ZIP/DMG manifest before upload. Any mismatch stops the job.

## Validation status
The edited YAML was checked for the original anchor commands and re-fetched through the GitHub API after the write. **No GitHub Actions pass, PowerShell execution, macOS execution, physical install, live relay or cross-network test is claimed for this commit.** The checks need real Windows and macOS runner results.

## Follow-up
1. Review this PR's workflow results and repair any actual failure.
2. Generate and commit `Cargo.lock` using Cargo with legitimate registry access, then validate PR #5's `--locked` checks.
3. Finish consent/input cleanup, authenticated WSS relay and device-level tests. Keep all artifacts UNTESTED until verified.
