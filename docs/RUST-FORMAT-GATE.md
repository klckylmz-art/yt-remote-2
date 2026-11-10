# Rust formatting CI gate

The Ubuntu `session-and-relay` job runs `cargo fmt --all -- --check` before compilation. A formatting failure blocks dependent Windows x64, macOS Intel, and macOS Apple Silicon jobs; do not bypass it by removing the check. Run `cargo fmt --all` on a development checkout, review the resulting Rust-only diff, and commit it. Then run `cargo fmt --all -- --check` locally and confirm the corresponding GitHub Actions step passes.

Evidence required: the exact workflow run, successful format step, and successful downstream build/test jobs. Adding this gate does **not** establish network interoperability, macOS permissions, installer usability, or a production release. Existing experimental artifacts remain untested on physical machines.

Baseline observed on 2026-11-10: workflow run 42765869932 reported success for Ubuntu, Windows x64, macOS Intel and macOS Apple Silicon, with UNTESTED artifacts; it predates this new formatting gate. Do not treat that run as evidence that this new check passed.
