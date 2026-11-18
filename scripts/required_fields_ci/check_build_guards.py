#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--root", required=True, type=Path)
root = parser.parse_args().root.resolve(strict=True)
owner = json.loads((root / "owner.json").read_text())
assert owner["root"] == owner["cleanup_scope"] == str(root) and owner["hosted_target"] is None
work = root / "build-guards"
work.mkdir()
results = []
for package in ["router", "payment_methods"]:
    for index, features in enumerate([["required_fields_sync"], ["v1", "v2", "required_fields_sync"]]):
        args = ["rustc", "--edition=2021", *[arg for feature in features for arg in ["--cfg", f'feature="{feature}"']],
                f"crates/{package}/build.rs", "-o", str(work / f"{package}-{index}")]
        result = subprocess.run(args, text=True, capture_output=True, timeout=60)
        assert result.returncode != 0 and "required_fields_sync supports v1 only" in result.stderr
        results.append({"package": package, "features": features, "expected_refusal": True})
(work / "report.json").write_text(json.dumps(results, indent=2) + "\n")
print(json.dumps({"expected_refusals": len(results), "scope": "actual_build_script_only"}))
