# Cargo.lock bootstrap evidence — 2026-11-10

## Confirmed baseline
- `Cargo.lock` on main returned GitHub Contents API 404.
- GitHub Actions run 42607323960: Ubuntu service succeeded; Windows x64 desktop build and artifact upload succeeded; macOS arm64 and Intel compiled and packaged, but the architecture verification step failed with `lipo` argument ordering (job 129126836247). The current main workflow has a corrected ordering, but that correction has not been verified by a new successful run.
- PR #5 introduces `--locked` without a committed lockfile, so it cannot be merged as-is.

## What this PR does
- Generates a real candidate `Cargo.lock` with Cargo on the Ubuntu runner, verifies it via `cargo metadata --locked`, and uploads it as a seven-day **candidate** artifact.
- Generates/validates a candidate lockfile on each desktop runner so the current source can continue through existing compilation, test and packaging gates without claiming dependency reproducibility.
- This is a **temporary bootstrap**, not a release-quality substitute for committing the lockfile. Dependency versions may still drift across runner jobs until the exact generated lockfile is committed.

## Next mandatory gate
1. Inspect this PR's actual Actions jobs and download the Ubuntu `Cargo-lock-CANDIDATE-<sha>` artifact.
2. Commit the generated lockfile in a reviewed PR (no hand-authored or fabricated lock entries).
3. Replace both generation steps with `test -s Cargo.lock` and use `--locked` on all cargo check/test/build commands; reconcile PR #5 first to avoid conflicting CI edits.
4. Verify Windows/macOS Intel/ARM builds, macOS architecture/signature checks, and SHA256 artifacts on the new SHA.
5. Continue consent, relay, native input-release and physical cross-network tests. No production release until all gates pass.

No new CI pass, physical install, deployed relay, or measured FPS is claimed by this document.
