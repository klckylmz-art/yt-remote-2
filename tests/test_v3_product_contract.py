#!/usr/bin/env python3
"""Prevent accidental account-based UX or premature release claims in v3 spec."""
import pathlib
import unittest

SPEC = pathlib.Path(__file__).resolve().parents[1] / "docs" / "YT-REMOTE-3-REQUIREMENTS.md"


class ProductContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.spec = SPEC.read_text(encoding="utf-8")

    def test_accountless_native_only(self):
        for required in ("No Google sign-in", "registration", "Windows 10/11 x64",
                         "macOS Intel and Apple Silicon", "No user-facing IP"):
            self.assertIn(required, self.spec)

    def test_explicit_consent_and_stop(self):
        for required in ("ALLOW SCREEN & KEYBOARD", "DENY", "CANCEL",
                         "STOP SHARING NOW", "before **any** frame or input",
                         "release", "Old tokens"):
            self.assertIn(required, self.spec)

    def test_no_unverified_release_claim(self):
        self.assertIn("has **not yet been integrated**", self.spec)
        self.assertIn("physical tests pass", self.spec)
        self.assertIn("AGPL-3.0", self.spec)


if __name__ == "__main__":
    unittest.main()
