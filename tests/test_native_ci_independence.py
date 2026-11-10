"""Guard native CI from being skipped when Linux session tests fail."""
import pathlib
import unittest

WORKFLOW = pathlib.Path(__file__).resolve().parents[1] / ".github/workflows/ci.yml"


class NativeBuildIndependenceTests(unittest.TestCase):
    def test_native_jobs_do_not_depend_on_linux(self):
        content = WORKFLOW.read_text(encoding="utf-8")
        desktop = content.split("  desktop-build:", 1)[1].split("\n  ", 1)[0]
        self.assertNotIn("needs: session-and-relay", desktop)
        self.assertIn("runs-on: ${{ matrix.os }}", desktop)
        for runner in ("windows-latest", "macos-15", "macos-15-intel"):
            self.assertIn(runner, desktop)

    def test_rust_format_is_checked(self):
        self.assertIn("cargo fmt --all -- --check", WORKFLOW.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
