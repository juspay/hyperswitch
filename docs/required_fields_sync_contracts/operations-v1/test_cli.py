#!/usr/bin/env python3
"""Actual router entry points, synthetic owned files and OS-denied networking only."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--router", required=True, type=Path)
    args = parser.parse_args()
    root = args.root.resolve(strict=True)
    owner = json.loads((root / "owner.json").read_text())
    assert owner["purpose"] == "Step 13 isolated command checks"
    assert owner["root"] == owner["cleanup_scope"] == str(root)
    assert owner["hosted_target"] is None and owner["owner"]
    binary = args.router.resolve(strict=True)
    registration = json.loads((root / "binary-registration.json").read_text())
    assert registration["binary"] == str(binary)
    binary_hash = hashlib.file_digest(binary.open("rb"), "sha256").hexdigest()
    assert registration["sha256"] == binary_hash
    work = root / "binary-operations"
    work.mkdir()
    policy = "(version 1)(allow default)(deny network*)"
    env = {"PATH": "/usr/bin:/bin", "TMPDIR": str(root), "RUN_ENV": "development",
           "ROUTER__MASTER_DATABASE__PORT": "not-a-port",
           "ROUTER__REDIS__PORT": "not-a-port"}
    probe = subprocess.run(["sandbox-exec", "-p", policy, sys.executable, "-c",
        "import socket\ns=socket.socket()\ntry:\n s.bind(('127.0.0.1',0))\nexcept PermissionError:\n raise SystemExit(0)\nraise SystemExit(1)"],
        capture_output=True, env=env, timeout=10)
    assert probe.returncode == 0, "network denial must be demonstrated first"
    results = []
    private = "synthetic-sensitive-never-log"

    def invoke(name, argv, code, exit_code):
        result = subprocess.run(["sandbox-exec", "-p", policy, str(binary), *argv],
            capture_output=True, text=True, env=env, cwd=root, timeout=30)
        output = result.stdout + result.stderr
        assert private not in output and str(root) not in output, name
        assert result.returncode == exit_code, (name, result.returncode, output)
        report = json.loads(output)
        assert report["code"] == code, (name, report)
        assert report["superposition_mutations_dispatched"] == 0
        assert report["leases_acquired"] == 0 and report["hosted_apply"] == "disabled"
        results.append({"case": name, "exit": result.returncode, "code": code})

    inputs = root / "operation-inputs"
    apply = json.loads((inputs / "apply.json").read_text())
    restore = json.loads((inputs / "restore.json").read_text())
    status = json.loads((inputs / "status.json").read_text())
    digest = "sha256:" + "1" * 64
    run = restore["planning"]["intent"]["recovery"]["from_run"]
    specs = [
        ("status", status, ["--run", run]),
        ("apply", apply, ["--intent", "1", "--plan-hash", digest]),
        ("resume", restore, ["--run", run, "--intent", "1", "--plan-hash", digest]),
        ("restore-plan", restore, ["--from-run", run, "--intent", "1", "--plan"]),
        ("restore-apply", restore, ["--from-run", run, "--intent", "1", "--plan-hash", digest]),
    ]
    for name, config, tail in specs:
        command = "restore" if name.startswith("restore-") else name
        path = work / f"{name}.json"
        path.write_text(json.dumps(config))

        def argv(suffix):
            return ["required-fields", command, "--target", config["target"]["target_id"],
                    "--command-config", str(path), "--output", str(work / f"{name}-{suffix}.report.json"), *tail]

        invoke(name + "-uncertified", argv("pending"), "certification_pending", 2)
        report = work / f"{name}-pending.report.json"
        assert os.stat(report).st_mode & 0o777 == 0o600
        invoke(name + "-existing-output", argv("pending"), "output_unavailable", 2)
        for flag in ["--force", "--host", "--token", "--skip-verification"]:
            invoke(name + flag, argv("flag") + [flag, private], "input_invalid", 2)
        malformed = copy.deepcopy(config)
        malformed["writer_token"] = private
        path.write_text(json.dumps(malformed))
        invoke(name + "-unknown-field", argv("unknown"), "input_invalid", 2)
        path.write_text('{"schema_version":1,"schema_version":1}')
        invoke(name + "-duplicate-json", argv("duplicate"), "input_invalid", 2)
        disabled = copy.deepcopy(config)
        disabled.pop("mode")
        path.write_text(json.dumps(disabled))
        invoke(name + "-default-mode", argv("disabled"),
               "certification_pending" if name == "status" else "disabled", 2)
        if name != "status":
            planning = copy.deepcopy(config)
            planning["mode"] = "plan"
            path.write_text(json.dumps(planning))
            invoke(name + "-plan-mode", argv("plan-mode"),
                   "certification_pending" if name == "restore-plan" else "disabled", 2)
            wrong = copy.deepcopy(config)
            wrong["planning"]["intent"]["generation"] += 1
            path.write_text(json.dumps(wrong))
            invoke(name + "-stale-intent", argv("intent"), "not_authoritative", 3)
        path.write_text(json.dumps(config))
        invoke(name + "-global-serving-config", ["--config-path", private, *argv("serving")], "input_invalid", 2)
        invoke(name + "-malformed-digest", [x if x != run else private for x in argv("digest")]
               if name != "apply" else [x if x != digest else private for x in argv("digest")], "input_invalid", 2)

    for command in ["render", "plan", "status", "apply", "resume", "restore", "seed", "reset"]:
        invoke(command + "-missing-arguments", ["required-fields", command], "input_invalid", 2)
    invoke("restore-ambiguous-mode", ["required-fields", "restore", "--target", apply["target"]["target_id"],
        "--command-config", str(inputs / "restore.json"), "--output", str(work / "ambiguous.json"),
        "--from-run", run, "--intent", "1", "--plan", "--plan-hash", digest], "input_invalid", 2)
    result = {"scope": "actual_binary_denials_only", "owner": owner["owner"],
              "binary_sha256": binary_hash, "network": "OS denied", "cases": results,
              "hosted_target": None, "certification": "pending"}
    (root / "binary-results.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"cases_passed": len(results), "binary_sha256": binary_hash}))


if __name__ == "__main__":
    main()
