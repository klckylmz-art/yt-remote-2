#!/usr/bin/env python3
"""Validate the RustDesk upstream adoption manifest without third-party dependencies."""
import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "rustdesk-upstream.json"
REQUIRED_BLOCKED = {"audio", "camera", "file-transfer", "clipboard-sync", "unattended-access"}
REQUIRED_GATES = {"license_review", "security_review", "windows_x64_verified",
                  "macos_intel_verified", "macos_arm_verified",
                  "cross_network_verified", "performance_benchmarked"}


class UpstreamPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = json.loads(MANIFEST.read_text(encoding="utf-8"))

    def test_source_is_pinned_before_import(self):
        upstream = self.policy["upstream"]
        self.assertEqual(upstream["repository"], "https://github.com/rustdesk/rustdesk")
        self.assertEqual(upstream["license"], "AGPL-3.0")
        if self.policy["integration"]["vendored_code"]:
            self.assertRegex(upstream["revision"] or "", r"^[0-9a-f]{40}$")
            self.assertEqual(upstream["review_status"], "approved")

    def test_no_unsafe_features_approved(self):
        integration = self.policy["integration"]
        self.assertTrue(REQUIRED_BLOCKED.issubset(set(integration["prohibited_features"])))
        self.assertFalse(set(integration["approved_components"]) &
                         set(integration["prohibited_features"]))

    def test_release_gates_are_explicit(self):
        gates = self.policy["release_gates"]
        self.assertTrue(REQUIRED_GATES.issubset(gates))
        self.assertTrue(all(type(value) is bool for value in gates.values()))
        if not all(gates.values()):
            self.assertNotEqual(self.policy["integration"]["mode"], "production")


if __name__ == "__main__":
    unittest.main()
