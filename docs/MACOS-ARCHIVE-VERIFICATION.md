# macOS archive smoke test (experimental builds)

The CI workflow now runs `bash packaging/macos/verify-distributables.sh "$PLATFORM"` after creating the ZIP/DMG and before uploading artifacts. Supported labels: `macOS-Apple-Silicon` (arm64) and `macOS-Intel` (x86_64).

The script extracts the distributed ZIP with `ditto`, attaches the distributed DMG read-only using `hdiutil`, and independently checks the executable path, `CFBundleExecutable`, Mach-O architecture with `lipo`, and ad-hoc code signature with `codesign --verify --deep --strict` in **both** archives. It unmounts the DMG and removes temporary files on exit. A failure blocks artifact upload. This is stronger than validating only the staging `.app`, which could differ from the delivered archives.

**Verification limits:** The shell script passes `bash -n` syntax validation on Linux. macOS commands have not yet been executed on a macOS runner for this change; the new workflow result is pending. These checks do not launch the application, validate Gatekeeper approval, confirm screen/accessibility permissions, test a physical Mac, or prove a cross-network session. No release gate is complete on this evidence alone.

**Baseline evidence:** Workflow run `42607323960` compiled and tested both Mac targets, but failed in the previous `lipo -verify_arch` argument order at the bundle validation step. The corrected invocation is present in current `main`; its successful rerun still needs confirmation. CI may separately be blocked by GitHub account billing before starting, which must not be confused with a Rust or packaging error.

**Operator verification on macOS:** Run `bash packaging/macos/verify-distributables.sh macOS-Apple-Silicon` or `macOS-Intel` after `packaging/macos/package.sh` for the corresponding architecture. Later, use real clean Macs for launch, consent, STOP, screen recording and accessibility tests.
