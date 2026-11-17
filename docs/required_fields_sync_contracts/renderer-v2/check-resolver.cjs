// Local-only bridge from actual Rust-rendered synthetic bundles to the pinned U JS reader.
// W's evaluator verifies all Step 1 source hashes before evaluating in a fresh VM realm.
// Explicit common-before-specific order is a fixture, not a server ordering certification.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const [webRoot, fixtureRoot, processId] = process.argv.slice(2);
assert(webRoot && fixtureRoot && /^\d+$/.test(processId), "expected W_ROOT OWNED_ROOT EXPORT_PID");
const owner = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "owner.json"), "utf8"));
assert.equal(owner.purpose, "Step 4 offline synthetic renderer/build checks");
const pack = JSON.parse(fs.readFileSync(path.join(__dirname, "../sdk-v1/cases.json"), "utf8"));
const { evaluator } = require(path.join(path.resolve(webRoot), "playwright-tests/required-fields-sync/evaluator.cjs"));
const evaluate = evaluator(pack, path.resolve(webRoot));
const key = "dynamic_fields.card.number._dropdown_options";
const required = "dynamic_fields.card.number._is_required";
const context = { connector: "adyen", payment_method: "Card", payment_method_type: "Debit", mandate_type: "mandate" };

function config(name, defaults) {
  const bundle = JSON.parse(fs.readFileSync(path.join(fixtureRoot, `${name}-${processId}.json`), "utf8"));
  assert.equal(bundle.schema_version, 2);
  assert.equal(bundle.scope.retirement_authority, "not_granted");
  const result = { default_configs: defaults, contexts: [], overrides: {}, dimensions: {} };
  for (const [condition, managed] of bundle.desired.contexts) {
    const id = `synthetic-${result.contexts.length}`;
    result.contexts.push({ id, condition, override_with_keys: [id], priority: 0, weight: 0 });
    result.overrides[id] = Object.fromEntries(Object.entries(managed).map(([k, v]) => [k, v.value]));
  }
  return result;
}
function check(config, supplied, expected) {
  const actual = evaluate({ config, context: supplied });
  assert.deepEqual(actual.errors, []);
  assert.deepEqual(actual.value, expected);
}

const inheritance = config("inheritance", { [key]: ["A", "B"], [required]: false });
assert.equal(inheritance.contexts.length, 2);
assert.equal(Object.hasOwn(inheritance.contexts[0].condition, "mandate_type"), false);
check(inheritance, context, { [key]: ["A", "B"], [required]: true });
check(inheritance, { ...context, mandate_type: "non_mandate" }, { [key]: ["A"], [required]: true });
check(inheritance, { ...context, payment_method: "card" }, { [key]: ["A", "B"], [required]: false });
delete inheritance.overrides["synthetic-1"][key];
check(inheritance, context, { [key]: ["A"], [required]: true });
check(config("empty", { [required]: true }), context, { [required]: true });
check(config("ordered", { [key]: [], [required]: false }), context,
  { [key]: ["B", "A", "A", "é", "Checking"], [required]: true });
// The JS reader adds override-only keys. The actual Rust resolver differs, as Step 1 records.
check(config("ordered", {}), context,
  { [key]: ["B", "A", "A", "é", "Checking"], [required]: true });
process.stdout.write(JSON.stringify({ schema_version: 1, layer: "actual-pinned-js-resolver", assertions: 7, status: "passed", browser: "not-executed", server_order: "not-certified" }) + "\n");
