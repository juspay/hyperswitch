"""Compile copied actual enum/match source, then require rejection of a future variant.

This narrow negative probe is separate from actual-package build/test evidence.
It never edits the real enum, Cargo workspace, dependency sources or shared cache.
"""
import json
from pathlib import Path
import subprocess
import sys

repository, fixture = map(Path, sys.argv[1:])
owner = json.loads((fixture / "owner.json").read_text())
assert owner["purpose"] == "Step 4 offline synthetic renderer/build checks"


def block(source, signature):
    start = source.index(signature)
    opening = source.index("{", start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


enum = block((repository / "crates/api_models/src/enums.rs").read_text(), "pub enum FieldType {")
function = block((repository / "crates/payment_methods/src/configs/required_fields_renderer.rs").read_text(), "pub fn field_options(")
probe = fixture / "exhaustiveness"
probe.mkdir()  # Refuse reuse; cleanup belongs to the registered fixture owner.
for name, declaration in [("control", enum), ("future", enum[:-1] + "FutureRequiredFieldTest,\n}")]:
    path = probe / f"{name}.rs"
    path.write_text(declaration + "\n" + function + "\n")
    result = subprocess.run(["rustc", "--edition=2021", "--crate-type=lib", str(path), "-o", str(probe / f"{name}.rlib")], capture_output=True, text=True, check=False)
    (probe / f"{name}.log").write_text(result.stdout + result.stderr)
    if name == "control":
        assert result.returncode == 0, result.stderr
    else:
        assert result.returncode != 0
        assert "E0004" in result.stderr and "FutureRequiredFieldTest" in result.stderr
print(json.dumps({"control": "compiled", "future_variant": "refused-E0004", "scope": "copied-actual-enum-and-field-options-only"}))
