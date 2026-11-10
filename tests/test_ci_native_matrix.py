"""CI regression: native platform jobs must not be gated by Linux failures."""
from pathlib import Path
import re
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/ci.yml"


class NativeMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_native_jobs_independent_of_linux(self):
        # Match job-level lines, not steps or comments.
        native = self.workflow.split("\n  desktop-build:", 1)[1].split("\n  ", 1)[0]
        self.assertNotRegex(native, r"(?m)^    needs:")
        self.assertNotIn("needs: session-and-relay", self.workflow)

    def test_all_required_native_runners(self):
        for runner in ("windows-latest", "macos-15", "macos-15-intel"):
            self.assertIn("os: " + runner, self.workflow)
        for target in ("x86_64-pc-windows-msvc", "aarch64-apple-darwin",
                       "x86_64-apple-darwin"):
            self.assertIn(target, self.workflow)

    def test_actual_build_and_artifact_checks_remain(self):
        for required in ("cargo fmt --all -- --check", "cargo check --workspace --all-targets",
                         "cargo test --workspace", "cargo build --release -p yt-remote-desktop",
                         "if-no-files-found: error", "SHA256SUMS"):
            self.assertIn(required, self.workflow)


if __name__ == "__main__":
    unittest.main()
