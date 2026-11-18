"""Offline guards, not server/browser certification. No fixture resources are written."""
import copy
import io
import json
from pathlib import Path
import re
import tarfile
import tomllib
import unittest

from prepare_browser import H, artifact_members, lock, sha
from test_seed_cli import canonical, profile_hash


class Contracts(unittest.TestCase):
    def test_pinned_verifier_artifact_is_complete_and_tampering_fails(self):
        pins = lock()
        data = (H / pins["artifact"]).read_bytes()
        self.assertEqual(len(artifact_members(pins, data)), 30)
        with self.assertRaises(AssertionError):
            artifact_members(pins, data + b"changed")
        bad = copy.deepcopy(pins)
        bad["artifact_files"].pop()
        with self.assertRaises(AssertionError):
            artifact_members(bad, data)

    def test_even_a_rehashed_archive_cannot_escape_its_fixture(self):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w") as archive:
            info = tarfile.TarInfo("../escape")
            info.size = 1
            archive.addfile(info, io.BytesIO(b"x"))
        pins = {"artifact_sha256": sha(data.getvalue()), "artifact_files": [{"path": "../escape", "sha256": sha(b"x")}]}
        with self.assertRaises(AssertionError):
            artifact_members(pins, data.getvalue())

    def test_named_local_banks_and_profile_identity_are_explicit(self):
        profile = json.loads((H / "config/required_fields_sync/local-profile.json").read_text())
        policy = json.loads((H / "config/required_fields_sync/local-seed-policy.json").read_text())
        development = tomllib.loads((H / "config/development.toml").read_text())
        banks = development["bank_config"]["eps"]["stripe"]["banks"].split(",")
        self.assertEqual(profile["inputs"], {"kind": "seed", "stripe_eps_banks": sorted(banks)})
        self.assertEqual(len(set(banks)), 27)
        self.assertEqual(policy["profile_hash"], profile_hash(profile))
        self.assertEqual(policy["scope"]["status"], "refused")  # Current approval gate, not a passing seed check.
        self.assertEqual(profile["catalog"]["unrestricted_countries"]["status"], "refused")
        self.assertEqual(profile["exclusions"]["sources"], [])
        evidence = json.loads((H / "docs/required_fields_sync_contracts/seed-v1/local-profile-source.json").read_text())
        for row in evidence["source_files"]:
            self.assertEqual(sha((H / row["path"]).read_bytes()), row["sha256"], row["path"])
        self.assertEqual(profile["release"]["source_tree_hash"], "sha256:" + sha(canonical(evidence["source_files"])))

    def test_current_server_candidates_are_not_silently_unified(self):
        for path, expected in [("docker-compose.yml", ["0.113.0"]), ("docker-compose-development.yml", ["0.115.5"]),
                               (".github/workflows/cypress-tests-runner.yml", ["0.102.0"] * 5)]:
            found = re.findall(r"image:\s*ghcr\.io/juspay/superposition-demo:([0-9.]+)", (H / path).read_text())
            self.assertEqual(found, expected, path)
        sdk = [p["version"] for p in tomllib.loads((H / "Cargo.lock").read_text())["package"] if p["name"] == "superposition_sdk"]
        self.assertEqual(sdk, ["0.119.0"])

    def test_manual_seed_and_deja_contract_are_retained(self):
        seed = tomllib.loads((H / "config/superposition_seed.toml").read_text())
        self.assertIs(seed["default-configs"]["deja_record"]["value"], False)
        uploader = (H / "scripts/seed_superposition.sh").read_text()
        for probe in ["/payments", "/health", "/foo/payments"]:
            self.assertIn(f'resolve_deja_record "{probe}"', uploader)
        self.assertIn('"LOCAL_COHORT": "path"', uploader)
        self.assertTrue(any("country" in row["_context_"] for row in seed["overrides"]))


if __name__ == "__main__":
    unittest.main()
