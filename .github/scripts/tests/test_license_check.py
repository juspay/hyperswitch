#!/usr/bin/env python3
"""Unit tests for `.github/scripts/license_check.py`.

Run with `python -m unittest discover --start-directory .github/scripts/tests`.
The tests never touch the network: license resolution is stubbed out and the
repository is replaced by an in-memory revision.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import license_check as lc  # noqa: E402

POLICY = {
    "settings": {"fail_on": ["deny"], "comment_on": ["deny", "warn", "unknown"]},
    "allow": {
        "licenses": [
            "MIT",
            "Apache-2.0",
            "Apache-2.0 WITH LLVM-exception",
            "BSD-3-Clause",
            "Unicode-3.0",
        ]
    },
    "warn": {"licenses": ["MPL-2.0", "CC-BY-4.0"]},
    "deny": {
        "licenses": [
            "GPL-3.0-only",
            "GPL-2.0-or-later",
            "AGPL-3.0-only",
            "CC-BY-NC-*",
            "GPL-2.0*",
        ]
    },
}


class FakeRevision(lc.Revision):
    """An in-memory stand-in for a git revision."""

    def __init__(self, files: dict[str, str]) -> None:
        self._contents = files
        self._files = sorted(files)
        self.ref = "fake"

    def verify(self) -> str:
        return "0" * 40

    def read(self, path: str) -> str | None:
        return self._contents.get(path)

    def files(self) -> list[str]:
        return list(self._files)


class StubResolver(lc.LicenseResolver):
    """Returns canned licenses instead of querying a registry."""

    def __init__(self, licenses: dict[str, str | None]) -> None:
        super().__init__(offline=True)
        self.licenses = licenses
        self.calls: list[str] = []

    def resolve(self, package: lc.Package) -> tuple[str | None, str | None]:
        self.calls.append(package.name)
        declared = self.licenses.get(package.name)
        if declared is None:
            return None, "npm metadata declares no license"
        return declared, None


def package(name: str, version: str = "1.0.0", **kwargs) -> lc.Package:
    kwargs.setdefault("ecosystem", "cargo")
    kwargs.setdefault("source", "registry+https://github.com/rust-lang/crates.io-index")
    return lc.Package(name=name, version=version, **kwargs)


class TestSpdxTokenizer(unittest.TestCase):
    def test_splits_parentheses_from_identifiers(self):
        self.assertEqual(
            lc.tokenize_spdx("(MIT OR Apache-2.0) AND ISC"),
            ["(", "MIT", "OR", "Apache-2.0", ")", "AND", "ISC"],
        )

    def test_expands_the_legacy_slash_form(self):
        self.assertEqual(
            lc.tokenize_spdx("MIT/Apache-2.0/ISC"),
            ["MIT", "OR", "Apache-2.0", "OR", "ISC"],
        )


class TestSpdxParser(unittest.TestCase):
    def test_and_binds_tighter_than_or(self):
        self.assertEqual(
            lc.parse_spdx("MIT OR ISC AND BSD-3-Clause"),
            ("or", ("id", "MIT"), ("and", ("id", "ISC"), ("id", "BSD-3-Clause"))),
        )

    def test_parentheses_override_precedence(self):
        self.assertEqual(
            lc.parse_spdx("(MIT OR ISC) AND BSD-3-Clause"),
            ("and", ("or", ("id", "MIT"), ("id", "ISC")), ("id", "BSD-3-Clause")),
        )

    def test_with_binds_tightest(self):
        self.assertEqual(
            lc.parse_spdx("MIT AND Apache-2.0 WITH LLVM-exception"),
            ("and", ("id", "MIT"), ("with", ("id", "Apache-2.0"), "LLVM-exception")),
        )

    def test_rejects_malformed_expressions(self):
        for expression in ("MIT AND", "OR MIT", "(MIT", "MIT)", ""):
            with self.subTest(expression=expression), self.assertRaises(ValueError):
                lc.parse_spdx(expression)


class TestPolicyClassification(unittest.TestCase):
    def setUp(self):
        self.policy = lc.Policy(POLICY, "test-policy.toml")

    def assert_tier(self, expression, expected):
        self.assertEqual(self.policy.tier_for_expression(expression), expected)

    def test_single_identifiers(self):
        self.assert_tier("MIT", lc.TIER_ALLOW)
        self.assert_tier("MPL-2.0", lc.TIER_WARN)
        self.assert_tier("GPL-3.0-only", lc.TIER_DENY)
        self.assert_tier("Beerware", lc.TIER_UNKNOWN)

    def test_matching_is_case_insensitive(self):
        self.assert_tier("mit", lc.TIER_ALLOW)
        self.assert_tier("gpl-3.0-ONLY", lc.TIER_DENY)

    def test_or_picks_the_least_restrictive_operand(self):
        # A dual-licensed crate may be taken under its permissive option.
        self.assert_tier("MIT OR Apache-2.0", lc.TIER_ALLOW)
        self.assert_tier("GPL-3.0-only OR MIT", lc.TIER_ALLOW)
        self.assert_tier("GPL-3.0-only OR MPL-2.0", lc.TIER_WARN)
        self.assert_tier("GPL-3.0-only OR Beerware", lc.TIER_UNKNOWN)

    def test_and_picks_the_most_restrictive_operand(self):
        # Every operand's obligations have to be honoured at once.
        self.assert_tier("MIT AND Apache-2.0", lc.TIER_ALLOW)
        self.assert_tier("MIT AND MPL-2.0", lc.TIER_WARN)
        self.assert_tier("MIT AND GPL-3.0-only", lc.TIER_DENY)
        self.assert_tier("MIT AND Beerware", lc.TIER_UNKNOWN)

    def test_nested_expressions(self):
        self.assert_tier("(MIT OR GPL-3.0-only) AND Unicode-3.0", lc.TIER_ALLOW)
        self.assert_tier("(MIT AND GPL-3.0-only) OR MPL-2.0", lc.TIER_WARN)

    def test_listed_exception_is_matched_as_a_whole(self):
        self.assert_tier("Apache-2.0 WITH LLVM-exception", lc.TIER_ALLOW)

    def test_unlisted_exception_does_not_soften_its_license(self):
        self.assert_tier(
            "GPL-2.0-or-later WITH Classpath-exception-2.0", lc.TIER_DENY
        )

    def test_deprecated_plus_suffix_maps_to_or_later(self):
        self.assert_tier("GPL-2.0+", lc.TIER_DENY)

    def test_wildcards_match_a_license_family(self):
        self.assert_tier("CC-BY-NC-SA-4.0", lc.TIER_DENY)
        self.assert_tier("CC-BY-4.0", lc.TIER_WARN)

    def test_most_restrictive_tier_wins_on_overlap(self):
        overlapping = lc.Policy(
            {"allow": {"licenses": ["GPL-2.0-only"]}, "deny": {"licenses": ["GPL-2.0*"]}}
        )
        self.assertEqual(
            overlapping.tier_for_expression("GPL-2.0-only"), lc.TIER_DENY
        )

    def test_undeclared_and_non_spdx_licenses_are_unknown(self):
        self.assert_tier(None, lc.TIER_UNKNOWN)
        self.assert_tier("", lc.TIER_UNKNOWN)
        self.assert_tier("SEE LICENSE IN LICENSE.md", lc.TIER_UNKNOWN)
        self.assert_tier("NOASSERTION", lc.TIER_UNKNOWN)
        self.assert_tier("UNLICENSED", lc.TIER_UNKNOWN)
        self.assert_tier("MIT AND", lc.TIER_UNKNOWN)

    def test_rejects_an_unknown_tier_in_settings(self):
        with self.assertRaises(lc.CheckError):
            lc.Policy({"settings": {"fail_on": ["nope"]}, "allow": {"licenses": ["MIT"]}})

    def test_rejects_an_incomplete_exception(self):
        with self.assertRaises(lc.CheckError):
            lc.Policy({"allow": {"licenses": ["MIT"]}, "exceptions": [{"name": "x"}]})


class TestPolicyExceptions(unittest.TestCase):
    def setUp(self):
        self.policy = lc.Policy(
            {
                **POLICY,
                "exceptions": [
                    {
                        "ecosystem": "cargo",
                        "name": "ring",
                        "versions": "0.17.*",
                        "tier": "allow",
                        "license": "ISC AND MIT AND OpenSSL",
                        "reason": "LICENSE file reviewed",
                    }
                ],
            }
        )

    def test_matches_name_version_and_ecosystem(self):
        self.assertIsNotNone(self.policy.exception_for(package("ring", "0.17.8")))
        self.assertIsNone(self.policy.exception_for(package("ring", "0.16.20")))
        self.assertIsNone(
            self.policy.exception_for(package("ring", "0.17.8", ecosystem="npm"))
        )

    def test_exception_short_circuits_registry_lookup(self):
        resolver = StubResolver({"ring": "GPL-3.0-only"})
        findings = lc.classify(
            [(package("ring", "0.17.8"), None)], self.policy, resolver
        )
        self.assertEqual(resolver.calls, [])
        self.assertEqual(findings[0].tier, lc.TIER_ALLOW)
        self.assertIn("LICENSE file reviewed", findings[0].note)


CARGO_MANIFESTS = {
    "Cargo.toml": '[workspace]\nmembers = ["crates/*"]\n',
    "crates/router/Cargo.toml": """
[package]
name = "router"

[dependencies]
serde = "1.0"
renamed = { package = "actual-crate", version = "1.0" }

[target.'cfg(unix)'.dependencies]
nix = "0.29"

[dev-dependencies]
wiremock = "0.6"
test_utils = { path = "../test_utils" }

[build-dependencies]
prost-build = "0.13"
""",
    "crates/test_utils/Cargo.toml": """
[package]
name = "test_utils"

[dependencies]
thirtyfour = "0.32"
""",
}

CARGO_LOCK = """
version = 4

[[package]]
name = "router"
version = "0.1.0"
dependencies = ["actual-crate", "nix", "prost-build", "serde", "test_utils", "wiremock"]

[[package]]
name = "test_utils"
version = "0.1.0"
dependencies = ["thirtyfour"]

[[package]]
name = "serde"
version = "1.0.210"
source = "registry+https://github.com/rust-lang/crates.io-index"
dependencies = ["serde_derive"]

[[package]]
name = "serde_derive"
version = "1.0.210"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "actual-crate"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "nix"
version = "0.29.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "prost-build"
version = "0.13.3"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "wiremock"
version = "0.6.2"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "thirtyfour"
version = "0.32.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "unused"
version = "9.9.9"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "git-dep"
version = "0.1.0"
source = "git+https://github.com/example/git-dep?rev=abc#abc123"
"""


class TestCargoCollector(unittest.TestCase):
    def setUp(self):
        self.revision = FakeRevision({**CARGO_MANIFESTS, "Cargo.lock": CARGO_LOCK})
        self.collector = lc.CargoCollector(
            "Cargo.lock", ["Cargo.toml", "crates/*/Cargo.toml"]
        )
        self.inventory = self.collector.collect(self.revision)
        self.scopes = {name: p.scope for (_, name, _), p in self.inventory.items()}

    def test_normal_dependencies_are_runtime(self):
        self.assertEqual(self.scopes["serde"], lc.SCOPE_RUNTIME)
        self.assertEqual(self.scopes["serde_derive"], lc.SCOPE_RUNTIME)
        self.assertEqual(self.scopes["nix"], lc.SCOPE_RUNTIME)

    def test_renamed_dependencies_resolve_to_the_real_crate(self):
        self.assertEqual(self.scopes["actual-crate"], lc.SCOPE_RUNTIME)

    def test_dev_and_build_dependencies_are_separated(self):
        self.assertEqual(self.scopes["wiremock"], lc.SCOPE_DEV)
        self.assertEqual(self.scopes["prost-build"], lc.SCOPE_DEV)

    def test_a_test_only_workspace_crate_does_not_ship_its_dependencies(self):
        # `test_utils` is only ever a dev dependency, so `thirtyfour` is not
        # runtime even though it is a normal dependency of `test_utils`.
        self.assertEqual(self.scopes["thirtyfour"], lc.SCOPE_DEV)

    def test_workspace_crates_are_not_inventoried(self):
        self.assertNotIn("router", self.scopes)
        self.assertNotIn("test_utils", self.scopes)

    def test_unreferenced_lockfile_entries_are_ignored(self):
        self.assertNotIn("unused", self.scopes)

    def test_git_dependencies_have_no_registry(self):
        collector = lc.CargoCollector("Cargo.lock", ["Cargo.toml"])
        entry = {"name": "git-dep", "version": "0.1.0", "source": "git+https://x"}
        inventory: dict = {}
        collector._add(inventory, entry, lc.SCOPE_RUNTIME)
        self.assertIsNone(next(iter(inventory.values())).registry)

    def test_a_missing_lockfile_yields_an_empty_inventory(self):
        self.assertEqual(self.collector.collect(FakeRevision({})), {})

    def test_scope_can_be_pinned_by_configuration(self):
        collector = lc.CargoCollector(
            "Cargo.lock", ["Cargo.toml", "crates/*/Cargo.toml"], force_scope=lc.SCOPE_DEV
        )
        scopes = {p.scope for p in collector.collect(self.revision).values()}
        self.assertEqual(scopes, {lc.SCOPE_DEV})

    def test_rejects_an_unknown_scope(self):
        with self.assertRaises(lc.CheckError):
            lc.CargoCollector("Cargo.lock", [], force_scope="production")


NPM_LOCK_V3 = json.dumps(
    {
        "lockfileVersion": 3,
        "packages": {
            "": {"name": "root", "version": "1.0.0"},
            "node_modules/lodash": {
                "version": "4.17.21",
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
            },
            "node_modules/mocha": {
                "version": "10.7.3",
                "dev": True,
                "resolved": "https://registry.npmjs.org/mocha/-/mocha-10.7.3.tgz",
            },
            "node_modules/@scope/pkg": {
                "version": "2.0.0",
                "devOptional": True,
                "resolved": "https://registry.npmjs.org/@scope/pkg/-/pkg-2.0.0.tgz",
            },
            "node_modules/aliased": {
                "name": "real-package",
                "version": "3.1.0",
                "resolved": "https://registry.npmjs.org/real-package/-/rp-3.1.0.tgz",
            },
            "node_modules/mocha/node_modules/lodash": {
                "version": "3.10.1",
                "dev": True,
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-3.10.1.tgz",
            },
            "packages/workspace-member": {"version": "1.0.0"},
            "node_modules/workspace-member": {"link": True, "resolved": "packages/x"},
        },
    }
)

NPM_LOCK_V1 = json.dumps(
    {
        "lockfileVersion": 1,
        "dependencies": {
            "newman": {
                "version": "git+ssh://git@github.com/knutties/newman.git#7106e194c15d",
                "dev": True,
            },
            "chalk": {
                "version": "4.1.2",
                "resolved": "https://registry.npmjs.org/chalk/-/chalk-4.1.2.tgz",
                "dependencies": {
                    "ansi-styles": {
                        "version": "4.3.0",
                        "resolved": (
                            "https://registry.npmjs.org/ansi-styles/-/"
                            "ansi-styles-4.3.0.tgz"
                        ),
                    }
                },
            },
        },
    }
)


class TestNpmCollector(unittest.TestCase):
    def collect(self, contents: str) -> dict:
        collector = lc.NpmCollector("package-lock.json")
        return collector.collect(FakeRevision({"package-lock.json": contents}))

    def test_lockfile_v3_scopes(self):
        inventory = self.collect(NPM_LOCK_V3)
        scopes = {(name, version): p.scope for (_, name, version), p in inventory.items()}
        self.assertEqual(scopes[("lodash", "4.17.21")], lc.SCOPE_RUNTIME)
        self.assertEqual(scopes[("mocha", "10.7.3")], lc.SCOPE_DEV)
        self.assertEqual(scopes[("@scope/pkg", "2.0.0")], lc.SCOPE_DEV)
        self.assertEqual(scopes[("lodash", "3.10.1")], lc.SCOPE_DEV)

    def test_lockfile_v3_resolves_aliases_to_the_published_name(self):
        names = {name for _, name, _ in self.collect(NPM_LOCK_V3)}
        self.assertIn("real-package", names)
        self.assertNotIn("aliased", names)

    def test_lockfile_v3_skips_the_root_and_workspace_members(self):
        names = {name for _, name, _ in self.collect(NPM_LOCK_V3)}
        self.assertNotIn("root", names)
        self.assertNotIn("workspace-member", names)

    def test_lockfile_v1_walks_nested_dependencies(self):
        inventory = self.collect(NPM_LOCK_V1)
        scopes = {name: p.scope for (_, name, _), p in inventory.items()}
        self.assertEqual(scopes["chalk"], lc.SCOPE_RUNTIME)
        self.assertEqual(scopes["ansi-styles"], lc.SCOPE_RUNTIME)
        self.assertEqual(scopes["newman"], lc.SCOPE_DEV)

    def test_lockfile_v1_git_dependencies_use_the_committish_as_version(self):
        inventory = self.collect(NPM_LOCK_V1)
        newman = next(p for (_, name, _), p in inventory.items() if name == "newman")
        self.assertEqual(newman.version, "7106e194c15d")
        self.assertIsNone(newman.registry)

    def test_registry_packages_are_resolvable(self):
        inventory = self.collect(NPM_LOCK_V3)
        lodash = inventory[("npm", "lodash", "4.17.21")]
        self.assertEqual(lodash.registry, "npmjs.com")

    def test_a_private_registry_is_not_treated_as_npmjs(self):
        contents = json.dumps(
            {
                "lockfileVersion": 3,
                "packages": {
                    "": {},
                    "node_modules/internal": {
                        "version": "1.0.0",
                        "resolved": "https://npm.internal.example.com/internal.tgz",
                    },
                },
            }
        )
        self.assertIsNone(next(iter(self.collect(contents).values())).registry)

    def test_invalid_json_is_reported(self):
        with self.assertRaises(lc.CheckError):
            self.collect("{not json")


class TestDelta(unittest.TestCase):
    @staticmethod
    def inventory(*packages: lc.Package) -> dict:
        return {p.key: p for p in packages}

    def test_reports_added_and_bumped_packages_only(self):
        base = self.inventory(
            package("unchanged", "1.0.0"),
            package("bumped", "1.0.0"),
        )
        head = self.inventory(
            package("unchanged", "1.0.0"),
            package("bumped", "2.0.0"),
            package("added", "0.1.0"),
        )
        delta = {p.name: previous for p, previous in lc.compute_delta(base, head)}
        self.assertEqual(delta, {"added": None, "bumped": "1.0.0"})

    def test_a_removed_package_is_not_reported(self):
        base = self.inventory(package("dropped", "1.0.0"))
        self.assertEqual(lc.compute_delta(base, {}), [])

    def test_the_same_name_in_two_ecosystems_is_kept_apart(self):
        base = self.inventory(package("shared", "1.0.0", ecosystem="npm"))
        head = self.inventory(
            package("shared", "1.0.0", ecosystem="npm"),
            package("shared", "1.0.0", ecosystem="cargo"),
        )
        delta = lc.compute_delta(base, head)
        self.assertEqual([p.ecosystem for p, _ in delta], ["cargo"])
        self.assertIsNone(delta[0][1])

    def test_an_empty_base_treats_everything_as_new(self):
        head = self.inventory(package("first", "1.0.0"))
        self.assertEqual(len(lc.compute_delta({}, head)), 1)


class TestClassify(unittest.TestCase):
    def setUp(self):
        self.policy = lc.Policy(POLICY, "test-policy.toml")

    def test_assigns_a_tier_per_package_and_sorts_worst_first(self):
        delta = [
            (package("permissive"), None),
            (package("copyleft"), None),
            (package("weak"), None),
            (package("undeclared"), None),
        ]
        resolver = StubResolver(
            {
                "permissive": "MIT OR Apache-2.0",
                "copyleft": "GPL-3.0-only",
                "weak": "MPL-2.0",
            }
        )
        findings = lc.classify(delta, self.policy, resolver, workers=2)
        self.assertEqual(
            [(f.package.name, f.tier) for f in findings],
            [
                ("copyleft", lc.TIER_DENY),
                ("undeclared", lc.TIER_UNKNOWN),
                ("weak", lc.TIER_WARN),
                ("permissive", lc.TIER_ALLOW),
            ],
        )

    def test_keeps_the_resolver_note_for_undeclared_licenses(self):
        findings = lc.classify(
            [(package("undeclared"), None)], self.policy, StubResolver({})
        )
        self.assertEqual(findings[0].note, "npm metadata declares no license")


class TestReporting(unittest.TestCase):
    def setUp(self):
        self.policy = lc.Policy(POLICY, "test-policy.toml")

    def render(self, findings, failed=False):
        return lc.render_markdown(findings, self.policy, failed=failed)

    def test_reports_each_tier_under_its_own_heading(self):
        findings = [
            lc.Finding(package("copyleft"), lc.TIER_DENY, "GPL-3.0-only"),
            lc.Finding(package("weak"), lc.TIER_WARN, "MPL-2.0"),
            lc.Finding(package("mystery"), lc.TIER_UNKNOWN, None),
            lc.Finding(package("fine"), lc.TIER_ALLOW, "MIT"),
        ]
        markdown = self.render(findings, failed=True)
        self.assertTrue(markdown.startswith(lc.COMMENT_MARKER))
        self.assertIn("Denied (1)", markdown)
        self.assertIn("Needs review (1)", markdown)
        self.assertIn("Unknown (1)", markdown)
        self.assertIn("blocks the merge", markdown)
        # Permissive packages stay collapsed rather than filling the comment.
        self.assertIn("<details>", markdown)
        self.assertIn("1 permissively licensed", markdown)

    def test_shows_the_previous_version_of_a_bumped_package(self):
        finding = lc.Finding(
            package("bumped", "2.0.0"), lc.TIER_ALLOW, "MIT", previous_version="1.0.0"
        )
        self.assertIn("(was `1.0.0`)", self.render([finding]))

    def test_escapes_pipes_so_the_table_survives(self):
        finding = lc.Finding(package("odd"), lc.TIER_UNKNOWN, "MIT | evil")
        row = [line for line in self.render([finding]).splitlines() if "odd" in line][0]
        self.assertEqual(row.count("|"), 7)

    def test_an_empty_delta_renders_a_short_report(self):
        self.assertIn("no new or updated dependencies", self.render([]))

    def test_a_huge_report_drops_the_permissive_listing_first(self):
        findings = [
            lc.Finding(package(f"crate-{index:05}"), lc.TIER_ALLOW, "MIT")
            for index in range(4000)
        ]
        findings.append(lc.Finding(package("copyleft"), lc.TIER_DENY, "GPL-3.0-only"))
        markdown = self.render(findings, failed=True)
        self.assertLessEqual(len(markdown), lc.MAX_COMMENT_LENGTH)
        # The finding that needs action survives; the long listing does not.
        self.assertIn("copyleft", markdown)
        self.assertIn("4000 further dependencies are permissively licensed", markdown)

    def test_a_report_that_is_still_too_long_is_truncated(self):
        findings = [
            lc.Finding(package(f"crate-{index:05}"), lc.TIER_DENY, "GPL-3.0-only")
            for index in range(4000)
        ]
        markdown = self.render(findings, failed=True)
        self.assertLessEqual(len(markdown), lc.MAX_COMMENT_LENGTH)
        self.assertIn("Report truncated", markdown)


class TestJsonReport(unittest.TestCase):
    def test_finding_serialises_every_field(self):
        finding = lc.Finding(
            package("crate", "1.2.3", scope=lc.SCOPE_DEV),
            lc.TIER_WARN,
            "MPL-2.0",
            note="checked",
            previous_version="1.2.2",
        )
        self.assertEqual(
            finding.to_json(),
            {
                "ecosystem": "cargo",
                "name": "crate",
                "version": "1.2.3",
                "previous_version": "1.2.2",
                "scope": lc.SCOPE_DEV,
                "license": "MPL-2.0",
                "tier": lc.TIER_WARN,
                "note": "checked",
            },
        )


class TestCommandLine(unittest.TestCase):
    def test_head_ref_defaults_to_the_working_tree(self):
        arguments = lc.parse_arguments(["--base-ref", "origin/main"])
        self.assertIsNone(arguments.head_ref)
        self.assertEqual(arguments.base_ref, "origin/main")

    def test_report_destinations_are_paths(self):
        arguments = lc.parse_arguments(
            [
                "--base-ref",
                "abc123",
                "--json-out",
                "out/report.json",
                "--markdown-out",
                "out/report.md",
                "--comment-flag-out",
                "out/should-comment",
                "--pr-number",
                "42",
            ]
        )
        self.assertEqual(arguments.json_out, Path("out/report.json"))
        self.assertEqual(arguments.markdown_out, Path("out/report.md"))
        self.assertEqual(arguments.comment_flag_out, Path("out/should-comment"))
        self.assertEqual(arguments.pr_number, 42)


class TestCollectorConfiguration(unittest.TestCase):
    def test_builds_one_collector_per_declared_ecosystem(self):
        collectors = lc.build_collectors(
            {
                "ecosystems": [
                    {"type": "cargo", "lockfile": "Cargo.lock"},
                    {"type": "npm", "lockfile": "web/package-lock.json"},
                ]
            }
        )
        self.assertEqual([c.ecosystem for c in collectors], ["cargo", "npm"])
        self.assertEqual(collectors[1].lockfile, "web/package-lock.json")

    def test_rejects_an_unsupported_ecosystem(self):
        with self.assertRaises(lc.CheckError):
            lc.build_collectors({"ecosystems": [{"type": "gradle"}]})

    def test_rejects_a_policy_with_no_ecosystems(self):
        with self.assertRaises(lc.CheckError):
            lc.build_collectors({})


class TestRepositoryPolicy(unittest.TestCase):
    """The policy this repository actually ships has to stay loadable."""

    @classmethod
    def setUpClass(cls):
        import tomllib

        cls.path = Path(__file__).resolve().parents[2] / "license-policy.toml"
        cls.data = tomllib.loads(cls.path.read_text(encoding="utf-8"))
        cls.policy = lc.Policy(cls.data, ".github/license-policy.toml")

    def test_declares_the_ecosystems_it_scans(self):
        self.assertTrue(lc.build_collectors(self.data))

    def test_classifies_the_licenses_named_in_the_policy_document(self):
        for expression, expected in (
            ("MIT", lc.TIER_ALLOW),
            ("Apache-2.0 WITH LLVM-exception", lc.TIER_ALLOW),
            ("Unicode-3.0", lc.TIER_ALLOW),
            ("MPL-2.0", lc.TIER_WARN),
            ("EPL-2.0", lc.TIER_WARN),
            ("GPL-3.0-or-later", lc.TIER_DENY),
            ("AGPL-3.0-only", lc.TIER_DENY),
            ("LGPL-2.1-only", lc.TIER_DENY),
            ("BUSL-1.1", lc.TIER_DENY),
            ("SSPL-1.0", lc.TIER_DENY),
            ("CC-BY-NC-SA-4.0", lc.TIER_DENY),
            ("GPL-2.0", lc.TIER_DENY),
            ("GPL-3.0+", lc.TIER_DENY),
            ("MIT OR Apache-2.0", lc.TIER_ALLOW),
            ("Apache-2.0 AND MIT", lc.TIER_ALLOW),
            ("GPL-3.0-only OR MIT", lc.TIER_ALLOW),
            ("MIT AND GPL-3.0-only", lc.TIER_DENY),
        ):
            with self.subTest(expression=expression):
                self.assertEqual(self.policy.tier_for_expression(expression), expected)

    def test_blocks_the_merge_only_on_denied_licenses(self):
        self.assertEqual(self.policy.fail_on, [lc.TIER_DENY])


if __name__ == "__main__":
    unittest.main()
