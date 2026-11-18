#!/usr/bin/env python3
"""Register a new disposable filesystem root. Does not register an API workspace."""
import argparse
import json
from pathlib import Path
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--root", type=Path, required=True)
parser.add_argument("--purpose", choices=["Step 14 isolated seed and CI checks", "Step 7 offline CLI checks"], required=True)
args = parser.parse_args()
args.root.absolute().mkdir()
root = args.root.resolve(strict=True)
marker = {"owner": str(uuid.uuid4()), "purpose": args.purpose, "root": str(root),
          "cleanup_scope": str(root), "hosted_target": None}
(root / "owner.json").write_text(json.dumps(marker, indent=2) + "\n")
print(json.dumps(marker))
