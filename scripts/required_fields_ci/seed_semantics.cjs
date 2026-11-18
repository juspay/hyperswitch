// Independent JS resolver oracle over actual generated synthetic seeds. No browser claim.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const [web, root] = process.argv.slice(2);
const { loadPack } = require(path.join(web, "playwright-tests/required-fields-sync/contract.cjs"));
const { evaluator } = require(path.join(web, "playwright-tests/required-fields-sync/evaluator.cjs"));
const evaluate = evaluator(loadPack().pack, web);
const required = "dynamic_fields.choice._is_required";
const options = "dynamic_fields.choice._dropdown_options";
function check(seed) {
  const config = { default_configs: {}, contexts: [], overrides: {} };
  for (const [key, row] of Object.entries(seed["default-configs"])) config.default_configs[key] = row.value;
  seed.overrides.forEach((row, index) => {
    const id = String(index), { _context_, ...values } = row;
    config.contexts.push({ id, condition: _context_, override_with_keys: [id] });
    config.overrides[id] = values;
  });
  const result = evaluate({ config, context: { connector: "a", payment_method: "Card", payment_method_type: "Debit" } });
  assert.deepEqual(result.errors, []);
  assert.equal(result.value[required], true);
  assert.deepEqual(result.value[options], ["A", "B"]); // Authored independently of the generator.
}
check(JSON.parse(fs.readFileSync(path.join(root, "good.json"))));
assert.throws(() => check(JSON.parse(fs.readFileSync(path.join(root, "wrong.json")))), assert.AssertionError);
process.stdout.write(JSON.stringify({ cases_passed: 2, scope: "actual_pinned_js_resolver_only", wrong_seed_agreement_rejected: true }) + "\n");
