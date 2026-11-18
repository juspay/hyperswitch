#!/usr/bin/env python3
"""Actual early seed CLI with compiled-source fixtures and independent literal expectations."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

H = Path(__file__).resolve().parents[2]
canonical = lambda v: json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def profile_hash(profile):
    return "sha256:" + hashlib.sha256(b"required-fields/inventory/v1\0planner-v1\0" + canonical(profile)).hexdigest()


def run(root, binary, exported):
    root = root.resolve(strict=True)
    owner = json.loads((root / "owner.json").read_text())
    assert owner["purpose"] == "Step 14 isolated seed and CI checks"
    assert owner["root"] == owner["cleanup_scope"] == str(root) and owner["hosted_target"] is None
    export_owner = json.loads((exported / "owner.json").read_text())
    assert export_owner["purpose"] == "Step 7 offline CLI checks" and export_owner["root"] == str(exported.resolve())
    work = root / "seed-cli"
    work.mkdir()
    binary = binary.resolve(strict=True)
    before_seed = (H / "config/superposition_seed.toml").read_bytes()
    before_tree = subprocess.check_output(["git", "-C", str(H), "status", "--porcelain=v1"])
    key = "dynamic_fields.card.number._is_required"
    condition = {"connector": "stripe", "payment_method": "Card", "payment_method_type": "Debit"}
    source = {"payment_method": "card", "payment_method_type": "debit", "connector": "stripe",
              "group": "common", "path": "payment_method_data.card.card_number"}
    profile = json.loads((exported / "seed-profile.json").read_text())
    selected = [row for row in profile["exclusions"]["sources"] if row["source"] == source]
    assert len(selected) == 1, "compiled fixture source changed; review the test expectation"
    profile["exclusions"]["sources"] = [row for row in profile["exclusions"]["sources"] if row["source"] != source]
    approved = lambda v: {"status": "approved", "value": v, "review": "synthetic-ci-only-not-business-approval"}
    profile["profile_id"] = "synthetic-single-source-seed"
    profile["policy"] = {"schema_version": 1, "contract_id": "synthetic-seed-policy",
                         "connectors": {"stripe": approved("stripe")}, "payment_methods": {"card": approved("Card")},
                         "payment_method_types": {"debit": approved("Debit")},
                         "bindings": [{"source": source, "field": approved("card.number"), "expected_required_field": source["path"]}]}
    profile["catalog"]["fields"] = {"card.number": approved({"render_kind": "text", "write_path": source["path"],
        "read_path": "card.card_number", "review": "synthetic-only", "source_types": ["user_card_number"], "default_options": None})}
    metadata = {"field_render_type": "CardNumber", "confirm_request_write_path": source["path"],
                "intent_data_read_path": "card.card_number", "default_label_text": "Fixture number", "field_display_order": 1}
    dimensions = {"connector": ["stripe"], "payment_method": ["Card"], "payment_method_type": ["Debit"], "mandate_type": ["mandate", "non_mandate"]}
    policy = json.loads((H / "config/required_fields_sync/local-seed-policy.json").read_text())
    policy["profile_hash"] = profile_hash(profile)
    policy["scope"] = approved({"grants": [{"condition": condition, "attributes": [key], "retire": [], "review": "synthetic-only"}],
        "dimensions": {name: {"position": i, "mandatory": False, "vocabulary": labels, "allow_enum_extension": False, "review": "synthetic-only"} for i, (name, labels) in enumerate(dimensions.items())},
        "fields": {"card.number": {"metadata": metadata, "allowed_options": None, "review": "synthetic-only"}}, "create_defaults": []})
    def put(name, value):
        file = work / name
        with file.open("x") as stream:
            stream.write(value if isinstance(value, str) else json.dumps(value))
        return file
    profile_path, policy_path = put("profile.json", profile), put("policy.json", policy)
    template = "# Manual seed fixture\n[default-configs]\ndeja_record = { value = false, schema = { type = 'boolean' } }\n"
    template += f'"{key}" = {{ value = false, schema = {{ type = "boolean" }} }}\n'
    for name, value in metadata.items():
        template += f'"dynamic_fields.card.number._{name}" = {{ value = {json.dumps(value)}, schema = {{ type = "{("integer" if isinstance(value, int) else "string")}" }} }}\n'
    for i, (name, labels) in enumerate(dimensions.items()):
        template += f'\n[dimensions.{name}]\nposition = {i}\ntype = "REGULAR"\nschema = {{ type = "string", enum = {json.dumps(labels)} }}\n'
    row = '\n[[overrides]]\n_context_ = { connector = "stripe", payment_method = "Card", payment_method_type = "Debit" }\n'
    template += row + f'"{key}" = false # approved synthetic change only\nmanual = ["B", "A", "A"]\n'
    template += '\n[[overrides]]\n_context_ = { country = "US", tenant = "synthetic" }\nmanual = true\n'
    template_path = put("template.toml", template)
    env = {"PATH": os.environ["PATH"], "TMPDIR": str(root), "RUN_ENV": "development",
           "ROUTER__MASTER_DATABASE__PORT": "not-a-port", "ROUTER__REDIS__PORT": "not-a-port"}
    results = []
    def invoke(name, argv, expected, extra=None):
        result = subprocess.run([str(binary), "required-fields", *map(str, argv)], cwd=H, env=env | (extra or {}), capture_output=True, text=True, timeout=60)
        assert result.returncode == expected, (name, result.returncode, result.stdout, result.stderr)
        text = result.stdout + result.stderr
        assert str(root) not in text and "synthetic-sensitive-never-log" not in text
        report = json.loads(text)
        results.append({"case": name, "exit": result.returncode, "report": report})
        return report
    def seed(template, tail, policy_file=policy_path):
        return ["seed", "--input-profile", profile_path, "--seed-policy", policy_file, "--template", template, *tail]
    rendered = work / "bundle.json"
    invoke("compiled-render", ["render", "--input-profile", profile_path, "--output", rendered], 0)
    bundle = json.loads(rendered.read_text())["bundle"]
    assert bundle["desired"]["contexts"] == [[condition, {key: {"kind": "required", "value": True}}]]
    first = work / "generated.toml"
    generated = invoke("generate", seed(template_path, ["--output", first]), 0)
    assert generated["report"]["superposition_mutations_dispatched"] == 0
    assert generated["report"]["leases_acquired"] == 0 and generated["uploader"] == "not_invoked"
    assert first.stat().st_mode & 0o777 == 0o600
    actual, original = tomllib.loads(first.read_text()), tomllib.loads(template)
    assert actual["overrides"][0][key] is True
    actual["overrides"][0][key] = False
    assert actual == original, "only the independently expected managed value may change"
    second = work / "repeat.toml"
    invoke("determinism-and-ambient-isolation", seed(template_path, ["--output", second]), 0, {"ROUTER__BANK_CONFIG__EPS__STRIPE__BANKS": "not-a-bank"})
    assert second.read_bytes() == first.read_bytes()
    snapshot = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in work.iterdir() if p.is_file()}
    invoke("check-clean", seed(first, ["--check"]), 0)
    drift = invoke("check-drift", seed(template_path, ["--check"]), 2)
    assert drift["code"] == "seed_drift"
    assert snapshot == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in work.iterdir() if p.is_file()}
    invoke("no-overwrite", seed(template_path, ["--output", first]), 2)
    duplicate = put("duplicate.toml", template + row + "manual = false\n")
    invoke("duplicate-condition", seed(duplicate, ["--check"]), 2)
    altered = copy.deepcopy(policy); altered["router_feature_keys"].append("V2")
    invoke("wrong-feature-profile", seed(first, ["--check"], put("wrong-policy.json", altered)), 2)
    pending = copy.deepcopy(policy); pending["scope"] = {"status": "refused", "reason": "synthetic-pending-review"}
    invoke("unapproved-scope", seed(first, ["--check"], put("pending-policy.json", pending)), 2)
    for name, inputs in [("ambient-profile-refused", {"kind": "effective_settings", "settings_file": "missing.toml", "expected_settings_hash": policy["profile_hash"]}),
                         ("implicit-banks-refused", {"kind": "seed", "stripe_eps_banks": None})]:
        invalid = copy.deepcopy(profile); invalid["inputs"] = inputs
        invoke(name, ["seed", "--input-profile", put(name + ".json", invalid), "--seed-policy", policy_path, "--template", first, "--check"], 2)
    local = invoke("complete-local-profile-is-blocked", ["seed", "--input-profile", H / "config/required_fields_sync/local-profile.json", "--seed-policy", H / "config/required_fields_sync/local-seed-policy.json", "--template", H / "config/superposition_seed.toml", "--check"], 2)
    assert local["code"] == "mapping_unresolved" and local["diagnostic_counts"]["MissingBinding"] > 0
    assert (H / "config/superposition_seed.toml").read_bytes() == before_seed
    assert subprocess.check_output(["git", "-C", str(H), "status", "--porcelain=v1"]) == before_tree
    report = {"scope": "actual_router_offline_seed_only", "binary_sha256": hashlib.file_digest(binary.open("rb"), "sha256").hexdigest(), "cases": results, "passed": len(results), "tracked_tree_unchanged": True, "complete_seed_approval": "pending"}
    (root / "seed-cli-report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": len(results), "complete_seed_approval": "pending"}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--router", type=Path, required=True)
    parser.add_argument("--exported", type=Path, required=True)
    args = parser.parse_args()
    run(args.root, args.router, args.exported)
