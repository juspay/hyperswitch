#!/usr/bin/env python3
"""Opt-in macOS binary checks. Only an already registered synthetic root may be used.

No server is started. sandbox-exec denies all network access, including localhost.
The full compiled profiles must be exported by the actual payment_methods tests first.
The plan fixture comes from the independently exercised synthetic command tests.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--router", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.absolute()
    owner = json.loads((root / "owner.json").read_text())
    assert owner["purpose"] == "Step 7 offline CLI checks"
    assert owner["root"] == str(root) == owner["cleanup_scope"]
    assert owner["owner"]
    canonical_root = root.resolve(strict=True)
    router = args.router.resolve(strict=True)
    assert router.is_relative_to(canonical_root), "binary must come from the owned build"
    assert sys.platform == "darwin", "requires an independently verified network-denial sandbox"
    sandbox = ["/usr/bin/sandbox-exec", "-p", "(version 1)(allow default)(deny network*)"]
    env = {"PATH": "/usr/bin:/bin", "TMPDIR": str(root), "RUN_ENV": "development"}
    # A local-only attempted connect must be denied by the OS, not merely refused by a port.
    probe = subprocess.run(
        sandbox + [sys.executable, "-c", "import socket,errno\ns=socket.socket()\ntry:\n s.connect(('127.0.0.1',1))\nexcept OSError as e:\n assert e.errno in (errno.EPERM,errno.EACCES), e\nelse:\n raise AssertionError('network was allowed')"],
        env=env, cwd=root, capture_output=True, timeout=10,
    )
    assert probe.returncode == 0, "network-denial preflight failed; do not run CLI checks"
    work = root / "binary-cli"
    work.mkdir()  # Explicitly refuses reuse; never deletes earlier artifacts.
    results = []
    sentinels = ("synthetic-sensitive-never-log", "synthetic-sensitive-argument", str(root), str(canonical_root))

    def invoke(name, arguments, expected, *, extra_env=None):
        result = subprocess.run(
            sandbox + [str(router)] + [str(v) for v in arguments],
            cwd=work, env=env | (extra_env or {}), capture_output=True, text=True, timeout=30,
        )
        output = result.stdout + result.stderr
        assert all(s not in output for s in sentinels), (name, "output leaked a sentinel/path")
        if expected == "help":
            assert result.returncode == 0 and "Usage:" in output, name
            assert "--input-profile" in output and "--output" in output, name
            envelope = {"outcome": "help"}
        else:
            envelope = json.loads(result.stdout if result.returncode == 0 else result.stderr)
            assert envelope.get("code", envelope["outcome"]) == expected, (name, envelope)
            assert (result.returncode == 0) == (expected == "rendered_offline"), name
            assert envelope["superposition_mutations_dispatched"] == 0, name
            assert envelope["leases_acquired"] == 0, name
            assert envelope["hosted_apply"] == "disabled", name
        results.append({"case": name, "exit": result.returncode, "envelope": envelope})
        return envelope

    def render_args(profile, output):
        return ["required-fields", "render", "--input-profile", profile, "--output", output]

    def put(name, value):
        path = work / name
        with path.open("x") as file:
            json.dump(value, file)
        return path

    seed = root / "seed-profile.json"
    effective = root / "effective-profile.json"
    seed_result = invoke("seed_without_services", render_args(seed, work / "seed.json"), "rendered_offline")
    polluted = invoke(
        "seed_ignores_ambient_bank_override", render_args(seed, work / "polluted.json"),
        "rendered_offline", extra_env={"ROUTER__BANK_CONFIG__EPS__STRIPE__BANKS": "not-a-real-bank"},
    )
    assert seed_result["bundle_hash"] == polluted["bundle_hash"]
    projected = invoke("effective_settings_without_services", render_args(effective, work / "effective.json"), "rendered_offline")
    assert seed_result["bundle_hash"] == projected["bundle_hash"]
    assert seed_result["profile_hash"] != projected["profile_hash"]
    bank_variants = []
    for name, banks in [("absent", None), ("empty", [])]:
        variant = json.loads(seed.read_text())
        variant["inputs"]["stripe_eps_banks"] = banks
        result = invoke("seed_" + name + "_banks", render_args(put(name + "-profile.json", variant), work / (name + "-banks.json")), "rendered_offline")
        bank_variants.append(result["bundle_hash"])
    assert len(set(bank_variants + [seed_result["bundle_hash"]])) == 3
    invalid_bank = json.loads(seed.read_text())
    invalid_bank["inputs"]["stripe_eps_banks"] = ["unknown_synthetic_bank"]
    invoke("seed_unknown_bank_refused", render_args(put("unknown-bank-profile.json", invalid_bank), work / "unknown-bank.json"), "input_invalid")
    assert not (work / "unknown-bank.json").exists()
    for path in [work / "seed.json", work / "polluted.json", work / "effective.json"]:
        artifact = json.loads(path.read_text())
        assert artifact["summary"]["sdk_verification"] == "not_executed"
        assert artifact["summary"]["provisional"]
        assert all(s not in path.read_text() for s in sentinels)
        assert path.stat().st_mode & 0o077 == 0

    invoke("effective_bank_drift_refused", render_args(effective, work / "drift.json"), "settings_mismatch",
           extra_env={"ROUTER__BANK_CONFIG__EPS__STRIPE__BANKS": "ing"})
    assert not (work / "drift.json").exists()
    missing = json.loads(effective.read_text())
    missing["inputs"]["settings_file"] = "missing-explicit-settings.toml"
    invoke("missing_effective_file_refused", render_args(put("missing-profile.json", missing), work / "missing.json"), "input_invalid")
    blocked = invoke("unapproved_compiled_profile_refused", render_args(root / "blocked-profile.json", work / "blocked.json"), "mapping_unresolved")
    assert sum(blocked["diagnostic_counts"].values()) > 0
    assert json.loads((work / "blocked.json").read_text())["outcome"] == "refused"
    original = (work / "seed.json").read_bytes()
    invoke("existing_output_preserved", render_args(seed, work / "seed.json"), "output_unavailable")
    assert original == (work / "seed.json").read_bytes()
    (work / "linked-profile.json").symlink_to(seed)
    invoke("symlink_input_refused", render_args(work / "linked-profile.json", work / "symlink.json"), "input_invalid")

    configs = []
    for path in root.glob("unit-cli-*/plan.json"):
        value = json.loads(path.read_text())
        if value.get("mode") == "plan" and value["target"] == value["ownership"]["target"]:
            configs.append(value)
    assert configs, "run registered synthetic command tests first"
    config = configs[0]
    config["reader_credential_file"] = "missing-reader-token-never-read"
    config_path = put("plan.json", config)
    target = config["target"]["target_id"]

    def plan_args(config_file, target_id=target):
        return ["required-fields", "plan", "--input-profile", seed, "--output", work / "plan-output.json",
                "--plan-config", config_file, "--target", target_id]

    invoke("unregistered_target_before_credentials", plan_args(config_path), "unsupported_server_contract")
    invoke("target_identity_mismatch", plan_args(config_path, "different-synthetic-target"), "target_mismatch")
    disabled = dict(config)
    del disabled["mode"]
    invoke("mode_defaults_disabled", plan_args(put("disabled.json", disabled)), "disabled")
    override = dict(config, endpoint="http://synthetic-sensitive-argument.invalid")
    invoke("unknown_target_override_refused", plan_args(put("override.json", override)), "input_invalid")
    assert not (work / "plan-output.json").exists()
    invoke("serving_config_conflict", ["-f", "synthetic-sensitive-argument"] + render_args(seed, work / "conflict.json"), "input_invalid")
    for command in ["apply", "seed", "status", "resume", "restore", "reset"]:
        invoke("unavailable_command_" + command, ["required-fields", command, "synthetic-sensitive-argument"], "input_invalid")
    for flag in ["--force", "--endpoint", "--org", "--workspace", "--header", "--token"]:
        invoke("rejected_override_" + flag[2:], render_args(seed, work / "invalid-args.json") + [flag, "synthetic-sensitive-argument"], "input_invalid")
    invoke("explicit_profile_required", ["required-fields", "render", "--output", work / "absent-profile.json"], "input_invalid")
    invoke("render_help", ["required-fields", "render", "--help"], "help")
    invoke("plan_help", ["required-fields", "plan", "--help"], "help")
    binary_hash = hashlib.sha256()
    with router.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            binary_hash.update(chunk)
    report = {
        "schema_version": 1,
        "scope": "actual_router_binary_offline_plumbing_not_server_or_browser_certification",
        "network": "OS_denied_all_network_verified_before_execution",
        "binary_sha256": binary_hash.hexdigest(),
        "cases": results,
        "passed": len(results),
        "fixture_owner": owner["owner"],
    }
    with (work / "report.json").open("x") as file:
        json.dump(report, file, indent=2)
        file.write("\n")
    print(json.dumps({"passed": len(results), "network": report["network"], "scope": report["scope"]}))


if __name__ == "__main__":
    main()
