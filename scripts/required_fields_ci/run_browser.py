#!/usr/bin/env python3
"""Run pinned source/semantic/browser checks inside an already provisioned isolated fixture."""
import argparse
import json
import os
from pathlib import Path
import subprocess

from prepare_browser import H, artifact_members, lock, sha


def run(root):
    root = root.resolve(strict=True)
    marker = json.loads((root / "fixture-target.json").read_text())
    assert marker["root"] == marker["cleanup_scope"] == str(root)
    assert marker["parent_step"] == 14 and marker["hosted_target"] is None and marker["server_mutations"] is False
    web = root / "web"
    assert marker["web"] == str(web) and not (web / ".git").exists()
    pins = lock()
    assert json.loads((root / "verifier-registration.json").read_text()) == pins
    for path, data in artifact_members(pins, (H / pins["artifact"]).read_bytes()).items():
        assert (web / path).read_bytes() == data, "verifier source changed"
    for row in pins["dependency_locks"]:
        assert sha((web / row["path"]).read_bytes()) == row["sha256"]
    assert subprocess.check_output(["node", "--version"], text=True).strip() == "v" + pins["node_version"]
    # The caller must also deny non-loopback networking at the OS boundary. Prove that boundary.
    probe = "import socket,errno\ns=socket.socket()\ntry:s.connect(('192.0.2.1',9))\nexcept OSError as e:assert e.errno in (errno.EPERM,errno.EACCES,errno.ENETUNREACH),e\nelse:raise AssertionError('network permitted')"
    subprocess.run(["python3", "-c", probe], check=True, timeout=5)
    env = {"PATH": os.environ["PATH"], "TMPDIR": str(root), "LANG": "en_US.UTF-8",
           "REQUIRED_FIELDS_CASE_PACK": str(H / pins["case_pack"]),
           "PLAYWRIGHT_BROWSERS_PATH": str(root / "browsers"),
           "RF_VERIFIER_INPUT": str(root / "verifier-input.json")}
    commands = [
        ["node", "--test", "playwright-tests/required-fields-sync/contract.test.cjs", "playwright-tests/required-fields-verifier/contract.test.cjs"],
        ["node", "node_modules/rescript/cli/rescript.js"],
        ["node", "playwright-tests/node_modules/typescript/bin/tsc", "-p", "playwright-tests/required-fields-sync/tsconfig.json", "--noEmit"],
        ["node", "playwright-tests/node_modules/typescript/bin/tsc", "-p", "playwright-tests/required-fields-verifier/tsconfig.json", "--noEmit"],
        ["node", "node_modules/webpack-cli/bin/cli.js", "--config", "playwright-tests/required-fields-sync/webpack.config.cjs"],
        ["node", "-e", "require('fs').writeFileSync(process.env.RF_VERIFIER_INPUT,JSON.stringify(require('./playwright-tests/required-fields-verifier/fixture-input.cjs').fixture()),{flag:'wx',mode:0o600})"],
        ["node", "playwright-tests/required-fields-verifier/run.cjs"],
    ]
    results = []
    for index, command in enumerate(commands):
        log = root / f"ci-command-{index}.log"
        with log.open("x") as output:
            result = subprocess.run(command, cwd=web, env=env, stdout=output, stderr=subprocess.STDOUT, timeout=900)
        results.append({"command": command, "exit": result.returncode, "log_sha256": sha(log.read_bytes())})
        (root / "ci-commands.json").write_text(json.dumps(results, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"Pinned browser command {index} failed; inspect its owned log")
    print(json.dumps({"scope": "synthetic_local_browser_only", "commands_passed": len(results), "target_certification": "pending"}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    run(parser.parse_args().root)
