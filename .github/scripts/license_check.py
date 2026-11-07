#!/usr/bin/env python3
"""Check the dependency licenses a pull request introduces against a policy.

The script compares the dependency lockfiles at two git revisions, keeps the
packages that are new or version-bumped at the head revision, resolves their
declared licenses from the public package registries and classifies each of
them against `.github/license-policy.toml`.

It deliberately never installs or builds anything: lockfiles are read straight
out of the git object database and licenses come from registry metadata. That
keeps the check fast, makes it safe to run against untrusted pull request
contents, and means a repository needs no language toolchain to adopt it.

Exit codes:
    0   no package landed in a tier listed under `settings.fail_on`
    1   at least one package did, and the merge should be blocked
    2   the check could not be completed
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import subprocess
import sys
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, Sequence

TIER_ALLOW = "allow"
TIER_WARN = "warn"
TIER_UNKNOWN = "unknown"
TIER_DENY = "deny"

# Ordered from the least to the most restrictive outcome. `OR` in an SPDX
# expression picks the minimum and `AND` picks the maximum of this ordering.
TIER_SEVERITY = {TIER_ALLOW: 0, TIER_WARN: 1, TIER_UNKNOWN: 2, TIER_DENY: 3}

TIER_LABELS = {
    TIER_DENY: "Denied",
    TIER_WARN: "Needs review",
    TIER_UNKNOWN: "Unknown",
    TIER_ALLOW: "Allowed",
}

TIER_ICONS = {
    TIER_DENY: "❌",
    TIER_WARN: "⚠️",
    TIER_UNKNOWN: "❓",
    TIER_ALLOW: "✅",
}

TIER_DESCRIPTIONS = {
    TIER_DENY: (
        "Strong-copyleft or source-available licenses. These block the merge: "
        "drop the dependency, replace it with a permissively licensed "
        "equivalent, or get an exception recorded in the policy file."
    ),
    TIER_WARN: (
        "Weak-copyleft licenses. These are allowed, but obligations such as "
        "publishing modifications to the dependency itself may apply."
    ),
    TIER_UNKNOWN: (
        "No license could be determined from registry metadata. Confirm the "
        "license by hand and, once verified, record it under `[[exceptions]]` "
        "in the policy file."
    ),
}

SCOPE_RUNTIME = "runtime"
SCOPE_DEV = "dev/build"
SCOPE_ORDER = {SCOPE_RUNTIME: 0, SCOPE_DEV: 1}

COMMENT_MARKER = "<!-- license-compliance-report -->"

# GitHub rejects issue comments larger than 65536 characters.
MAX_COMMENT_LENGTH = 60000

DEFAULT_USER_AGENT = (
    "hyperswitch-license-compliance (+https://github.com/juspay/hyperswitch)"
)


class CheckError(Exception):
    """A condition that stops the check from producing a verdict."""


# --------------------------------------------------------------------------- #
# SPDX expressions
# --------------------------------------------------------------------------- #

# `SEE LICENSE IN <file>` is an npm convention for a license that is not an
# SPDX identifier at all.
_NON_SPDX_PREFIXES = ("see license in", "see licence in")


def tokenize_spdx(expression: str) -> list[str]:
    """Split an SPDX expression into parentheses, operators and identifiers."""

    tokens: list[str] = []
    for raw in expression.replace("(", " ( ").replace(")", " ) ").split():
        if raw in {"(", ")"} or "/" not in raw:
            tokens.append(raw)
            continue

        # `MIT/Apache-2.0` is a legacy spelling of `MIT OR Apache-2.0` that old
        # crates and npm packages still declare.
        parts = [part for part in raw.split("/") if part]
        for index, part in enumerate(parts):
            if index:
                tokens.append("OR")
            tokens.append(part)
    return tokens


class SpdxParser:
    """Recursive descent parser for the subset of SPDX that registries emit.

    Grammar, tightest binding last::

        expression := term ("OR" term)*
        term       := factor ("AND" factor)*
        factor     := atom ("WITH" identifier)?
        atom       := identifier | "(" expression ")"
    """

    def __init__(self, tokens: Sequence[str]) -> None:
        self._tokens = list(tokens)
        self._position = 0

    def parse(self) -> tuple:
        if not self._tokens:
            raise ValueError("empty license expression")
        node = self._parse_expression()
        if self._position != len(self._tokens):
            raise ValueError(f"unexpected token {self._tokens[self._position]!r}")
        return node

    def _peek(self) -> str | None:
        if self._position < len(self._tokens):
            return self._tokens[self._position]
        return None

    def _advance(self) -> str:
        token = self._peek()
        if token is None:
            raise ValueError("unexpected end of license expression")
        self._position += 1
        return token

    def _parse_expression(self) -> tuple:
        node = self._parse_term()
        while (token := self._peek()) is not None and token.upper() == "OR":
            self._advance()
            node = ("or", node, self._parse_term())
        return node

    def _parse_term(self) -> tuple:
        node = self._parse_factor()
        while (token := self._peek()) is not None and token.upper() == "AND":
            self._advance()
            node = ("and", node, self._parse_factor())
        return node

    def _parse_factor(self) -> tuple:
        node = self._parse_atom()
        if (token := self._peek()) is not None and token.upper() == "WITH":
            self._advance()
            return ("with", node, self._advance())
        return node

    def _parse_atom(self) -> tuple:
        token = self._advance()
        if token == "(":
            node = self._parse_expression()
            closing = self._advance()
            if closing != ")":
                raise ValueError(f"expected ')' but found {closing!r}")
            return node
        if token in {")"} or token.upper() in {"OR", "AND", "WITH"}:
            raise ValueError(f"unexpected token {token!r}")
        return ("id", token)


def parse_spdx(expression: str) -> tuple:
    return SpdxParser(tokenize_spdx(expression)).parse()


def evaluate_spdx(node: tuple, classify_identifier: Callable[[str], str]) -> str:
    """Fold a parsed expression into a single tier.

    `OR` resolves to its least restrictive operand because a downstream user
    may pick either license; `AND` resolves to its most restrictive operand
    because every operand's terms have to be honoured.
    """

    kind = node[0]
    if kind == "id":
        return classify_identifier(node[1])
    if kind == "with":
        left, exception = node[1], node[2]
        if left[0] == "id":
            combined = classify_identifier(f"{left[1]} WITH {exception}")
            if combined != TIER_UNKNOWN:
                return combined
        # An unlisted exception must not soften the license it applies to.
        return evaluate_spdx(left, classify_identifier)
    if kind in {"or", "and"}:
        tiers = (
            evaluate_spdx(node[1], classify_identifier),
            evaluate_spdx(node[2], classify_identifier),
        )
        pick = min if kind == "or" else max
        return pick(tiers, key=lambda tier: TIER_SEVERITY[tier])
    raise ValueError(f"unsupported expression node {kind!r}")


# --------------------------------------------------------------------------- #
# Policy
# --------------------------------------------------------------------------- #


def _glob(value: str, pattern: str) -> bool:
    return fnmatch.fnmatchcase(value.lower(), pattern.lower())


@dataclass(frozen=True)
class PolicyException:
    """A hand-verified override for a single package."""

    name: str
    tier: str
    reason: str
    ecosystem: str | None = None
    versions: str = "*"
    license: str | None = None

    def matches(self, package: "Package") -> bool:
        if self.ecosystem is not None and self.ecosystem != package.ecosystem:
            return False
        return _glob(package.name, self.name) and _glob(package.version, self.versions)


class Policy:
    """The tier lists and settings loaded from the policy file."""

    def __init__(self, data: dict, source: str = "<memory>") -> None:
        self.source = source
        settings = data.get("settings", {})
        self.fail_on = list(settings.get("fail_on", [TIER_DENY]))
        self.comment_on = list(
            settings.get("comment_on", [TIER_DENY, TIER_WARN, TIER_UNKNOWN])
        )

        # Checked most restrictive first, so that an identifier matching both an
        # `allow` and a `deny` pattern is denied.
        self.tiers = {
            tier: list(data.get(tier, {}).get("licenses", []))
            for tier in (TIER_DENY, TIER_WARN, TIER_ALLOW)
        }
        if not any(self.tiers.values()):
            raise CheckError(f"{self.source} declares no licenses in any tier")

        self.exceptions: list[PolicyException] = []
        for index, entry in enumerate(data.get("exceptions", []), start=1):
            missing = {"name", "tier", "reason"} - entry.keys()
            if missing:
                raise CheckError(
                    f"exception #{index} in {self.source} is missing "
                    f"{', '.join(sorted(missing))}"
                )
            if entry["tier"] not in TIER_SEVERITY:
                raise CheckError(
                    f"exception #{index} in {self.source} has unknown tier "
                    f"{entry['tier']!r}"
                )
            self.exceptions.append(
                PolicyException(
                    name=entry["name"],
                    tier=entry["tier"],
                    reason=entry["reason"],
                    ecosystem=entry.get("ecosystem"),
                    versions=entry.get("versions", "*"),
                    license=entry.get("license"),
                )
            )

        for tier in (*self.fail_on, *self.comment_on):
            if tier not in TIER_SEVERITY:
                raise CheckError(f"unknown tier {tier!r} in {self.source} settings")

    def tier_for_identifier(self, identifier: str) -> str:
        candidates = [identifier]
        if identifier.endswith("+"):
            # `GPL-2.0+` is the deprecated spelling of `GPL-2.0-or-later`.
            base = identifier[:-1]
            candidates += [f"{base}-or-later", base]

        for tier in (TIER_DENY, TIER_WARN, TIER_ALLOW):
            for candidate in candidates:
                for pattern in self.tiers[tier]:
                    if _glob(candidate, pattern):
                        return tier
        return TIER_UNKNOWN

    def tier_for_expression(self, expression: str | None) -> str:
        if expression is None:
            return TIER_UNKNOWN
        text = expression.strip()
        if not text or text.lower().startswith(_NON_SPDX_PREFIXES):
            return TIER_UNKNOWN
        try:
            return evaluate_spdx(parse_spdx(text), self.tier_for_identifier)
        except ValueError:
            # A malformed expression is reported, never guessed at.
            return TIER_UNKNOWN

    def exception_for(self, package: "Package") -> PolicyException | None:
        for exception in self.exceptions:
            if exception.matches(package):
                return exception
        return None


# --------------------------------------------------------------------------- #
# Repository access
# --------------------------------------------------------------------------- #


class Revision:
    """Read-only view of the repository at a git revision."""

    def __init__(self, repo_root: Path, ref: str) -> None:
        self.repo_root = repo_root
        self.ref = ref
        self._files: list[str] | None = None

    def _git(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["git", "-C", str(self.repo_root), *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )

    def verify(self) -> str:
        result = self._git("rev-parse", "--verify", f"{self.ref}^{{commit}}")
        if result.returncode != 0:
            raise CheckError(
                f"cannot resolve git revision {self.ref!r}; "
                "the checkout may be too shallow"
            )
        return result.stdout.strip()

    def read(self, path: str) -> str | None:
        result = self._git("show", f"{self.ref}:{path}")
        if result.returncode != 0:
            return None
        return result.stdout

    def files(self) -> list[str]:
        if self._files is None:
            result = self._git("ls-tree", "-r", "--name-only", self.ref)
            if result.returncode != 0:
                raise CheckError(f"cannot list files at revision {self.ref!r}")
            self._files = result.stdout.splitlines()
        return self._files

    def glob(self, pattern: str) -> list[str]:
        return sorted(path for path in self.files() if _glob(path, pattern))


class WorkingTree(Revision):
    """The checked-out working tree, used for local runs."""

    def __init__(self, repo_root: Path) -> None:
        super().__init__(repo_root, "HEAD")

    def verify(self) -> str:
        return super().verify()

    def read(self, path: str) -> str | None:
        candidate = self.repo_root / path
        if not candidate.is_file():
            return None
        return candidate.read_text(encoding="utf-8")

    def files(self) -> list[str]:
        if self._files is None:
            result = self._git("ls-files")
            if result.returncode != 0:
                raise CheckError("cannot list files in the working tree")
            self._files = result.stdout.splitlines()
        return self._files


# --------------------------------------------------------------------------- #
# Package inventories
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class Package:
    ecosystem: str
    name: str
    version: str
    scope: str = SCOPE_RUNTIME
    source: str | None = None

    @property
    def key(self) -> tuple[str, str, str]:
        return (self.ecosystem, self.name, self.version)

    @property
    def registry(self) -> str | None:
        """The registry this package's license can be looked up in, if any."""

        if self.source is None:
            return None
        if self.ecosystem == "cargo" and self.source.startswith("registry+"):
            return "crates.io"
        if self.ecosystem == "npm" and "://" in self.source:
            host = urllib.parse.urlparse(self.source).hostname or ""
            if host == "registry.npmjs.org":
                return "npmjs.com"
        return None


class Collector:
    """Builds the package inventory of one ecosystem at one revision."""

    ecosystem = ""

    def __init__(self, force_scope: str | None = None) -> None:
        if force_scope is not None and force_scope not in SCOPE_ORDER:
            raise CheckError(
                f"unknown scope {force_scope!r}; expected one of "
                f"{', '.join(SCOPE_ORDER)}"
            )
        self.force_scope = force_scope

    def collect(self, revision: Revision) -> dict[tuple[str, str, str], Package]:
        raise NotImplementedError

    def _merge(
        self, inventory: dict[tuple[str, str, str], Package], package: Package
    ) -> None:
        """Keep the widest scope when a package is reachable more than once."""

        if self.force_scope is not None and package.scope != self.force_scope:
            package = Package(
                ecosystem=package.ecosystem,
                name=package.name,
                version=package.version,
                scope=self.force_scope,
                source=package.source,
            )
        existing = inventory.get(package.key)
        if existing is None or SCOPE_ORDER[package.scope] < SCOPE_ORDER[existing.scope]:
            inventory[package.key] = package


class CargoCollector(Collector):
    """Inventory from `Cargo.lock`, scoped using the workspace manifests.

    Cargo does not record dependency kinds in the lockfile, so the runtime set
    is derived by walking the lockfile graph from the `[dependencies]` of every
    workspace manifest. Anything the walk does not reach is only needed to
    build or test the workspace.
    """

    ecosystem = "cargo"

    _RUNTIME_SECTIONS = ("dependencies",)
    _DEV_SECTIONS = ("dev-dependencies", "build-dependencies")

    def __init__(
        self,
        lockfile: str,
        manifests: Sequence[str],
        force_scope: str | None = None,
    ) -> None:
        super().__init__(force_scope)
        self.lockfile = lockfile
        self.manifests = list(manifests)

    def collect(self, revision: Revision) -> dict[tuple[str, str, str], Package]:
        lock_text = revision.read(self.lockfile)
        if lock_text is None:
            return {}

        try:
            lock = tomllib.loads(lock_text)
        except tomllib.TOMLDecodeError as error:
            raise CheckError(f"{self.lockfile} is not valid TOML: {error}") from None

        by_id: dict[tuple[str, str], dict] = {}
        by_name: dict[str, list[tuple[str, str]]] = {}
        for entry in lock.get("package", []):
            name, version = entry.get("name"), entry.get("version")
            if not name or not version:
                continue
            identifier = (name, version)
            by_id[identifier] = entry
            by_name.setdefault(name, []).append(identifier)

        runtime_roots, dev_roots, members = self._roots(revision)
        runtime = self._reachable(runtime_roots, by_id, by_name, members)
        dev = self._reachable(dev_roots, by_id, by_name, members) - runtime

        inventory: dict[tuple[str, str, str], Package] = {}
        for identifiers, scope in ((runtime, SCOPE_RUNTIME), (dev, SCOPE_DEV)):
            for identifier in identifiers:
                self._add(inventory, by_id[identifier], scope)
        return inventory

    def _add(self, inventory: dict, entry: dict, scope: str) -> None:
        source = entry.get("source")
        if source is None:
            # A path dependency: one of the workspace's own crates.
            return
        self._merge(
            inventory,
            Package(
                ecosystem=self.ecosystem,
                name=entry["name"],
                version=entry["version"],
                scope=scope,
                source=source,
            ),
        )

    def _roots(self, revision: Revision) -> tuple[set[str], set[str], set[str]]:
        members: dict[str, tuple[set[str], set[str]]] = {}
        anonymous: tuple[set[str], set[str]] = (set(), set())
        seen: set[str] = set()

        for pattern in self.manifests:
            for path in revision.glob(pattern):
                if path in seen:
                    continue
                seen.add(path)
                text = revision.read(path)
                if text is None:
                    continue
                try:
                    manifest = tomllib.loads(text)
                except tomllib.TOMLDecodeError:
                    continue

                normal: set[str] = set()
                dev: set[str] = set()
                self._manifest_roots(manifest, normal, dev)
                name = manifest.get("package", {}).get("name")
                if isinstance(name, str):
                    members[name] = (normal, dev)
                else:
                    # A virtual workspace manifest declares no package of its own.
                    anonymous[0].update(normal)
                    anonymous[1].update(dev)

        shipped = self._shipped_members(members)

        runtime = set(anonymous[0])
        dev = set(anonymous[1])
        for name, (normal, member_dev) in members.items():
            # A crate that only ever appears as a dev dependency of another
            # workspace crate ships nothing, so neither do its dependencies.
            (runtime if name in shipped else dev).update(normal)
            dev.update(member_dev)

        # A crate reached through a normal dependency edge stays runtime even if
        # another manifest also lists it as a dev dependency.
        return runtime, dev - runtime, set(members)

    @staticmethod
    def _shipped_members(members: dict[str, tuple[set[str], set[str]]]) -> set[str]:
        """Workspace crates that are not exclusively test or benchmark support."""

        dev_referenced = set()
        for _, member_dev in members.values():
            dev_referenced |= member_dev & members.keys()

        shipped = {name for name in members if name not in dev_referenced}
        # A crate pulled in as a dev dependency somewhere may still be a normal
        # dependency of a crate that does ship, so grow the set to a fixpoint.
        queue = list(shipped)
        while queue:
            for dependency in members[queue.pop()][0] & members.keys():
                if dependency not in shipped:
                    shipped.add(dependency)
                    queue.append(dependency)
        return shipped

    def _manifest_roots(self, manifest: dict, runtime: set[str], dev: set[str]) -> None:
        def read(table: object, into: set[str]) -> None:
            if not isinstance(table, dict):
                return
            for key, value in table.items():
                # `foo = { package = "bar" }` renames `bar` to `foo` locally.
                if isinstance(value, dict) and isinstance(value.get("package"), str):
                    into.add(value["package"])
                else:
                    into.add(key)

        targets = manifest.get("target", {})
        tables = [manifest]
        if isinstance(targets, dict):
            tables.extend(targets.values())

        for table in tables:
            if not isinstance(table, dict):
                continue
            for section in self._RUNTIME_SECTIONS:
                read(table.get(section), runtime)
            for section in self._DEV_SECTIONS:
                read(table.get(section), dev)

    @staticmethod
    def _reachable(
        roots: Iterable[str],
        by_id: dict[tuple[str, str], dict],
        by_name: dict[str, list[tuple[str, str]]],
        members: set[str],
    ) -> set[tuple[str, str]]:
        seen: set[tuple[str, str]] = set()
        queue: list[tuple[str, str]] = []

        def push(identifier: tuple[str, str]) -> None:
            if identifier not in seen:
                seen.add(identifier)
                queue.append(identifier)

        for name in roots:
            for identifier in by_name.get(name, ()):
                push(identifier)

        while queue:
            entry = by_id[queue.pop()]
            # A workspace crate's lockfile edges merge its normal, dev and build
            # dependencies into one list, so walking them would drag test-only
            # crates into the runtime set. Its manifest is the authority
            # instead, and `_roots` has already seeded those dependencies.
            if entry.get("source") is None and entry["name"] in members:
                continue
            for specification in entry.get("dependencies", []):
                parts = specification.split()
                name = parts[0]
                if len(parts) > 1:
                    identifier = (name, parts[1])
                    if identifier in by_id:
                        push(identifier)
                    continue
                # An unversioned edge is unambiguous by construction, but stay
                # tolerant of hand-edited lockfiles and follow every candidate.
                for identifier in by_name.get(name, ()):
                    push(identifier)
        return seen


class NpmCollector(Collector):
    """Inventory from `package-lock.json`, lockfile versions 1 through 3."""

    ecosystem = "npm"

    _NODE_MODULES = "node_modules/"

    def __init__(self, lockfile: str, force_scope: str | None = None) -> None:
        super().__init__(force_scope)
        self.lockfile = lockfile

    def collect(self, revision: Revision) -> dict[tuple[str, str, str], Package]:
        text = revision.read(self.lockfile)
        if text is None:
            return {}

        try:
            lock = json.loads(text)
        except json.JSONDecodeError as error:
            raise CheckError(f"{self.lockfile} is not valid JSON: {error}") from None

        inventory: dict[tuple[str, str, str], Package] = {}
        if "packages" in lock:
            self._collect_v2(lock["packages"], inventory)
        else:
            self._collect_v1(lock.get("dependencies", {}), False, inventory)
        return inventory

    def _collect_v2(self, packages: dict, inventory: dict) -> None:
        for path, entry in packages.items():
            if not path or entry.get("link"):
                # The project root, and workspace members symlinked into it.
                continue
            index = path.rfind(self._NODE_MODULES)
            if index == -1:
                # A workspace member's own source directory.
                continue
            # `name` is set when the entry is an alias of another package.
            name = entry.get("name") or path[index + len(self._NODE_MODULES) :]
            version = entry.get("version")
            if not name or not version:
                continue
            scope = (
                SCOPE_DEV
                if entry.get("dev") or entry.get("devOptional")
                else SCOPE_RUNTIME
            )
            self._merge(
                inventory, self._package(name, version, scope, entry.get("resolved"))
            )

    def _collect_v1(
        self, dependencies: dict, inherited_dev: bool, inventory: dict
    ) -> None:
        for name, entry in dependencies.items():
            version = entry.get("version")
            if not version:
                continue
            is_dev = inherited_dev or bool(entry.get("dev"))
            scope = SCOPE_DEV if is_dev else SCOPE_RUNTIME
            self._merge(
                inventory, self._package(name, version, scope, entry.get("resolved"))
            )
            self._collect_v1(entry.get("dependencies", {}), is_dev, inventory)

    @classmethod
    def _package(
        cls, name: str, version: str, scope: str, resolved: str | None
    ) -> Package:
        # Lockfile v1 records a git dependency with its git URL as the version.
        if "://" in version:
            resolved = resolved or version
            _, _, committish = version.partition("#")
            version = committish[:12] if committish else version
        return Package(
            ecosystem=cls.ecosystem,
            name=name,
            version=version,
            scope=scope,
            source=resolved or "https://registry.npmjs.org/",
        )


COLLECTORS: dict[str, Callable[[dict], Collector]] = {
    "cargo": lambda config: CargoCollector(
        lockfile=config.get("lockfile", "Cargo.lock"),
        manifests=config.get("manifests", ["Cargo.toml"]),
        force_scope=config.get("scope"),
    ),
    "npm": lambda config: NpmCollector(
        lockfile=config.get("lockfile", "package-lock.json"),
        force_scope=config.get("scope"),
    ),
}


def build_collectors(policy_data: dict) -> list[Collector]:
    configured = policy_data.get("ecosystems", [])
    if not configured:
        raise CheckError("the policy file declares no `[[ecosystems]]` to scan")

    collectors: list[Collector] = []
    for index, config in enumerate(configured, start=1):
        kind = config.get("type")
        if kind not in COLLECTORS:
            raise CheckError(
                f"ecosystem #{index} has unsupported type {kind!r}; supported "
                f"types are {', '.join(sorted(COLLECTORS))}"
            )
        collectors.append(COLLECTORS[kind](config))
    return collectors


# --------------------------------------------------------------------------- #
# License resolution
# --------------------------------------------------------------------------- #


class LicenseResolver:
    """Resolves declared licenses from the public package registries."""

    def __init__(
        self,
        timeout: float = 20.0,
        attempts: int = 3,
        user_agent: str = DEFAULT_USER_AGENT,
        offline: bool = False,
    ) -> None:
        self.timeout = timeout
        self.attempts = attempts
        self.user_agent = user_agent
        self.offline = offline

    def resolve(self, package: Package) -> tuple[str | None, str | None]:
        """Return the declared license expression and an explanatory note."""

        registry = package.registry
        if registry is None:
            return None, "not published to a public registry"
        if self.offline:
            return None, "registry lookups disabled"

        try:
            if registry == "crates.io":
                return self._from_crates_io(package)
            return self._from_npm(package)
        except CheckError as error:
            return None, str(error)

    def _from_crates_io(self, package: Package) -> tuple[str | None, str | None]:
        url = (
            "https://crates.io/api/v1/crates/"
            f"{urllib.parse.quote(package.name)}/"
            f"{urllib.parse.quote(package.version)}"
        )
        payload = self._get_json(url)
        if payload is None:
            return None, "not found on crates.io"
        declared = (payload.get("version") or {}).get("license")
        if not declared:
            return None, "crates.io metadata carries a license file, not an SPDX id"
        return declared, None

    def _from_npm(self, package: Package) -> tuple[str | None, str | None]:
        url = (
            "https://registry.npmjs.org/"
            f"{urllib.parse.quote(package.name, safe='')}/"
            f"{urllib.parse.quote(package.version)}"
        )
        payload = self._get_json(url)
        if payload is None:
            return None, "not found on the npm registry"

        declared = payload.get("license")
        if isinstance(declared, dict):
            # Pre-SPDX manifests used `{"type": "MIT", "url": "..."}`.
            declared = declared.get("type")
        if not declared:
            legacy = payload.get("licenses")
            if isinstance(legacy, dict):
                legacy = [legacy]
            if isinstance(legacy, list):
                types = [
                    item.get("type")
                    for item in legacy
                    if isinstance(item, dict) and item.get("type")
                ]
                # A `licenses` array offers a choice between its entries.
                declared = " OR ".join(types) if types else None
        if not isinstance(declared, str) or not declared:
            return None, "npm metadata declares no license"
        return declared, None

    def _get_json(self, url: str) -> dict | None:
        request = urllib.request.Request(
            url, headers={"User-Agent": self.user_agent, "Accept": "application/json"}
        )
        last_error = "unknown error"
        for attempt in range(1, self.attempts + 1):
            try:
                with urllib.request.urlopen(request, timeout=self.timeout) as response:
                    return json.loads(response.read().decode("utf-8"))
            except urllib.error.HTTPError as error:
                if error.code == 404:
                    return None
                last_error = f"HTTP {error.code}"
                if error.code not in (429, 500, 502, 503, 504):
                    break
            except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
                last_error = str(error)
            if attempt < self.attempts:
                time.sleep(min(2**attempt, 8))
        raise CheckError(f"registry lookup failed ({last_error})")


# --------------------------------------------------------------------------- #
# Classification
# --------------------------------------------------------------------------- #


@dataclass
class Finding:
    package: Package
    tier: str
    license: str | None = None
    note: str | None = None
    previous_version: str | None = None

    def to_json(self) -> dict:
        return {
            "ecosystem": self.package.ecosystem,
            "name": self.package.name,
            "version": self.package.version,
            "previous_version": self.previous_version,
            "scope": self.package.scope,
            "license": self.license,
            "tier": self.tier,
            "note": self.note,
        }


def compute_delta(
    base: dict[tuple[str, str, str], Package],
    head: dict[tuple[str, str, str], Package],
) -> list[tuple[Package, str | None]]:
    """Return the head packages that the pull request adds or version-bumps."""

    versions_in_base: dict[tuple[str, str], list[str]] = {}
    for ecosystem, name, version in base:
        versions_in_base.setdefault((ecosystem, name), []).append(version)

    delta: list[tuple[Package, str | None]] = []
    for key, package in head.items():
        if key in base:
            continue
        previous = versions_in_base.get((package.ecosystem, package.name))
        delta.append((package, ", ".join(sorted(previous)) if previous else None))

    delta.sort(key=lambda item: (item[0].ecosystem, item[0].name.lower(), item[0].version))
    return delta


def classify(
    delta: Sequence[tuple[Package, str | None]],
    policy: Policy,
    resolver: LicenseResolver,
    workers: int = 8,
) -> list[Finding]:
    findings: list[Finding] = []
    pending: list[tuple[Package, str | None]] = []

    for package, previous in delta:
        exception = policy.exception_for(package)
        if exception is None:
            pending.append((package, previous))
            continue
        findings.append(
            Finding(
                package=package,
                tier=exception.tier,
                license=exception.license,
                note=f"policy exception: {exception.reason}",
                previous_version=previous,
            )
        )

    if pending:
        with ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
            resolved = list(pool.map(resolver.resolve, (item[0] for item in pending)))
        for (package, previous), (expression, note) in zip(pending, resolved):
            findings.append(
                Finding(
                    package=package,
                    tier=policy.tier_for_expression(expression),
                    license=expression,
                    note=note,
                    previous_version=previous,
                )
            )

    findings.sort(
        key=lambda finding: (
            -TIER_SEVERITY[finding.tier],
            SCOPE_ORDER[finding.package.scope],
            finding.package.ecosystem,
            finding.package.name.lower(),
            finding.package.version,
        )
    )
    return findings


# --------------------------------------------------------------------------- #
# Reporting
# --------------------------------------------------------------------------- #

_ESCAPED_PIPE = "\\|"


def _code(value: str | None) -> str:
    if not value:
        return "—"
    # A raw pipe would break out of the markdown table cell.
    return "`" + value.replace("|", _ESCAPED_PIPE) + "`"


def _version_cell(finding: Finding) -> str:
    if finding.previous_version:
        return f"`{finding.package.version}` (was `{finding.previous_version}`)"
    return f"`{finding.package.version}`"


def _table(findings: Sequence[Finding]) -> list[str]:
    lines = [
        "| Package | Version | License | Scope | Ecosystem |",
        "| --- | --- | --- | --- | --- |",
    ]
    for finding in findings:
        license_cell = _code(finding.license)
        if finding.note:
            license_cell = f"{license_cell} <sup>{finding.note}</sup>"
        lines.append(
            f"| `{finding.package.name}` | {_version_cell(finding)} | {license_cell} "
            f"| {finding.package.scope} | {finding.package.ecosystem} |"
        )
    return lines


def render_markdown(findings: Sequence[Finding], policy: Policy, failed: bool) -> str:
    """Render the sticky pull request comment, within GitHub's size limit."""

    report = _render_markdown(findings, policy, failed, collapse_allowed=True)
    if len(report) <= MAX_COMMENT_LENGTH:
        return report

    # Drop the collapsed list of permissive packages before anything that
    # somebody has to act on.
    report = _render_markdown(findings, policy, failed, collapse_allowed=False)
    if len(report) <= MAX_COMMENT_LENGTH:
        return report

    notice = "\n\n> Report truncated; see the workflow run for the full listing.\n"
    return report[: MAX_COMMENT_LENGTH - len(notice)].rstrip() + notice


def _render_markdown(
    findings: Sequence[Finding],
    policy: Policy,
    failed: bool,
    collapse_allowed: bool,
) -> str:
    by_tier: dict[str, list[Finding]] = {tier: [] for tier in TIER_SEVERITY}
    for finding in findings:
        by_tier[finding.tier].append(finding)

    total = len(findings)
    lines = [COMMENT_MARKER, "## \U0001f4dc Dependency license compliance", ""]

    if not total:
        lines += ["This pull request introduces no new or updated dependencies.", ""]
        return "\n".join(lines)

    lines += [
        f"Checked **{total}** dependenc{'y' if total == 1 else 'ies'} added or "
        f"updated by this pull request against `{policy.source}`.",
        "",
    ]

    for tier in (TIER_DENY, TIER_WARN, TIER_UNKNOWN):
        entries = by_tier[tier]
        if not entries:
            continue
        lines += [
            f"### {TIER_ICONS[tier]} {TIER_LABELS[tier]} ({len(entries)})",
            "",
            TIER_DESCRIPTIONS[tier],
            "",
            *_table(entries),
            "",
        ]

    allowed = by_tier[TIER_ALLOW]
    if allowed and collapse_allowed:
        lines += [
            "<details>",
            f"<summary>{TIER_ICONS[TIER_ALLOW]} {len(allowed)} permissively licensed "
            "(no action needed)</summary>",
            "",
            *_table(allowed),
            "",
            "</details>",
            "",
        ]
    elif allowed:
        lines += [
            f"{TIER_ICONS[TIER_ALLOW]} {len(allowed)} further dependenc"
            f"{'y is' if len(allowed) == 1 else 'ies are'} permissively licensed "
            "and need no action.",
            "",
        ]

    verdict = (
        "❌ **This check blocks the merge.**"
        if failed
        else "✅ **This check does not block the merge.**"
    )
    lines += [
        "---",
        "",
        f"{verdict} Only dependencies that this pull request adds or version-bumps "
        f"are evaluated. Tiers, and hand-verified exceptions, live in "
        f"`{policy.source}`.",
        "",
    ]
    return "\n".join(lines)


def render_terminal(findings: Sequence[Finding]) -> str:
    if not findings:
        return "No new or updated dependencies to check."

    name_width = max(len(finding.package.name) for finding in findings)
    version_width = max(len(finding.package.version) for finding in findings)
    lines = []
    for finding in findings:
        lines.append(
            f"{TIER_LABELS[finding.tier]:<13}"
            f"{finding.package.name:<{name_width}}  "
            f"{finding.package.version:<{version_width}}  "
            f"{finding.license or finding.note or 'unknown'}  "
            f"[{finding.package.scope}, {finding.package.ecosystem}]"
        )
    return "\n".join(lines)


def emit_annotations(findings: Sequence[Finding], fail_on: Sequence[str]) -> None:
    for finding in findings:
        if finding.tier not in fail_on:
            continue
        print(
            f"::error title=Denied license::The {finding.package.ecosystem} package "
            f"{finding.package.name} {finding.package.version} is licensed under "
            f"{finding.license or 'an undeclared license'}, which the license "
            "policy denies.",
            file=sys.stderr,
        )


# --------------------------------------------------------------------------- #
# Entry point
# --------------------------------------------------------------------------- #


def parse_arguments(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check the dependency licenses a pull request introduces."
    )
    parser.add_argument(
        "--base-ref",
        required=True,
        help="revision to compare against, usually the merge base",
    )
    parser.add_argument(
        "--head-ref",
        default=None,
        help="revision under review; defaults to the working tree",
    )
    parser.add_argument(
        "--repo-root", default=Path("."), type=Path, help="path to the repository"
    )
    parser.add_argument(
        "--policy",
        default=None,
        type=Path,
        help="policy file (default: <repo-root>/.github/license-policy.toml)",
    )
    parser.add_argument("--json-out", type=Path, help="write the machine report here")
    parser.add_argument(
        "--markdown-out", type=Path, help="write the pull request comment here"
    )
    parser.add_argument(
        "--comment-flag-out",
        type=Path,
        help=(
            "write `true` or `false` here, saying whether the comment is worth "
            "posting; lets a workflow decide without parsing JSON"
        ),
    )
    parser.add_argument(
        "--pr-number", type=int, help="pull request number recorded in the JSON report"
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="skip registry lookups; every license then resolves as unknown",
    )
    parser.add_argument(
        "--workers", type=int, default=8, help="parallel registry lookups"
    )
    return parser.parse_args(argv)


def run(arguments: argparse.Namespace) -> int:
    repo_root = arguments.repo_root.resolve()
    policy_path = arguments.policy or repo_root / ".github" / "license-policy.toml"

    try:
        policy_data = tomllib.loads(policy_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise CheckError(f"policy file not found: {policy_path}") from None
    except tomllib.TOMLDecodeError as error:
        raise CheckError(f"{policy_path} is not valid TOML: {error}") from None

    try:
        policy_name = policy_path.resolve().relative_to(repo_root).as_posix()
    except ValueError:
        policy_name = str(policy_path)
    policy = Policy(policy_data, policy_name)
    collectors = build_collectors(policy_data)

    base = Revision(repo_root, arguments.base_ref)
    head = (
        Revision(repo_root, arguments.head_ref)
        if arguments.head_ref
        else WorkingTree(repo_root)
    )
    base_sha, head_sha = base.verify(), head.verify()

    base_inventory: dict[tuple[str, str, str], Package] = {}
    head_inventory: dict[tuple[str, str, str], Package] = {}
    for collector in collectors:
        base_inventory |= collector.collect(base)
        head_inventory |= collector.collect(head)

    delta = compute_delta(base_inventory, head_inventory)
    print(
        f"{len(head_inventory)} dependencies at {head_sha[:12]}, "
        f"{len(base_inventory)} at {base_sha[:12]}: {len(delta)} added or updated.",
        file=sys.stderr,
    )

    resolver = LicenseResolver(offline=arguments.offline)
    findings = classify(delta, policy, resolver, workers=arguments.workers)
    blocking = [finding for finding in findings if finding.tier in policy.fail_on]
    reportable = [finding for finding in findings if finding.tier in policy.comment_on]

    emit_annotations(findings, policy.fail_on)
    print(render_terminal(findings))

    markdown = render_markdown(findings, policy, failed=bool(blocking))
    if arguments.markdown_out:
        arguments.markdown_out.write_text(markdown, encoding="utf-8")
    if arguments.comment_flag_out:
        arguments.comment_flag_out.write_text(
            "true" if reportable else "false", encoding="utf-8"
        )

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as summary:
            summary.write(markdown.replace(COMMENT_MARKER, "", 1).lstrip() + "\n")

    if arguments.json_out:
        arguments.json_out.write_text(
            json.dumps(
                {
                    "base_sha": base_sha,
                    "head_sha": head_sha,
                    "pr_number": arguments.pr_number,
                    "policy": policy_name,
                    "status": "fail" if blocking else "pass",
                    "should_comment": bool(reportable),
                    "counts": {
                        tier: sum(1 for f in findings if f.tier == tier)
                        for tier in TIER_SEVERITY
                    },
                    "findings": [finding.to_json() for finding in findings],
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )

    if blocking:
        print(
            f"\n{len(blocking)} denied license(s) introduced by this change.",
            file=sys.stderr,
        )
        return 1
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    try:
        return run(parse_arguments(argv))
    except CheckError as error:
        print(f"::error::{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
