"""Parse the actual bundle metadata without launching apps or requesting grants."""
import pathlib
import plistlib
import re
import shutil
import subprocess
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "packaging/macos/package.sh"


class MacBundleMetadataTests(unittest.TestCase):
    def test_generated_bundle_metadata(self):
        script = SCRIPT.read_text(encoding="utf-8")
        match = re.search(r"<<'PLIST'\n(.*?)\nPLIST(?:\n|$)", script, re.S)
        self.assertIsNotNone(match, "Packaging must supply a parseable plist")
        info = plistlib.loads(match.group(1).encode("utf-8"))
        self.assertEqual(info["CFBundleDisplayName"], "YT Remote 3.0")
        self.assertEqual(info["CFBundleName"], "YT Remote 3.0")
        self.assertEqual(info["CFBundleIdentifier"], "net.ytremote.desktop")
        self.assertEqual(info["CFBundleExecutable"], "YTRemote")
        self.assertEqual(info["CFBundleShortVersionString"], "3.0.0")
        self.assertRegex(info["CFBundleVersion"], r"^\d+(?:\.\d+){0,2}$")
        self.assertEqual(info["LSMinimumSystemVersion"], "12.0")
        self.assertIn("approve", info["NSScreenCaptureUsageDescription"])
        # These features are outside this product's scope.
        for key in ("NSCameraUsageDescription", "NSMicrophoneUsageDescription",
                    "NSAppleEventsUsageDescription"):
            self.assertNotIn(key, info)

    @unittest.skipUnless(shutil.which("bash"), "Bash is unavailable on this host")
    def test_shell_syntax(self):
        subprocess.run([shutil.which("bash"), "-n", str(SCRIPT)], check=True)


if __name__ == "__main__":
    unittest.main()
