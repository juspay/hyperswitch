// Offline comparison only. No browser acceptance, server certification or trusted receipt.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');

const [root, webRoot] = process.argv.slice(2);
assert(root && webRoot && path.isAbsolute(root) && path.isAbsolute(webRoot));
const owner = JSON.parse(fs.readFileSync(path.join(root, 'owner.json')));
assert.equal(owner.root, root);
assert.equal(owner.cleanup_scope, root);
assert.equal(owner.purpose, 'Step 6 offline synthetic planner checks');
const bytes = fs.readFileSync(path.join(root, 'planner-cases.json'));
assert(bytes.length <= 1024 * 1024);
const cases = JSON.parse(bytes);
assert.equal(cases.scope, 'synthetic_fixture');
assert.equal(cases.schema_version, 1);
assert.deepEqual(cases.profiles.map(p => p.id), ['unchanged', 'inherited-options', 'missing-default', 'connector-orders', 'mandate-inheritance']);
const pack = JSON.parse(fs.readFileSync(path.join(__dirname, '../sdk-v1/cases.json')));
const { evaluator } = require(path.join(webRoot, 'playwright-tests/required-fields-sync/evaluator.cjs'));
const evaluate = evaluator(pack, webRoot); // verifies every pinned W/U source hash first
const core = JSON.parse(fs.readFileSync(path.join(root, 'core-observations.json')));
assert.equal(core.length, cases.profiles.length);
const canonical = value => {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') return `{${Object.keys(value).sort().map(k => `${JSON.stringify(k)}:${canonical(value[k])}`).join(',')}}`;
  return JSON.stringify(value);
};
const digest = value => 'sha256:' + crypto.createHash('sha256')
  .update('required-fields/inventory/v1\0planner-v1\0').update(canonical(value)).digest('hex');
const relevant = (value, fields) => Object.fromEntries(Object.entries(value)
  .filter(([key]) => fields.some(field => key.startsWith(`dynamic_fields.${field}._`))));
let jsChecks = 0, rustChecks = 0;
for (const [profileIndex, profile] of cases.profiles.entries()) {
  const request = profile.request;
  assert.equal(request.contract_id, 'required-fields-planner-cases-v1');
  assert(Buffer.byteLength(JSON.stringify(request)) <= 262144);
  assert(request.contexts.length > 0 && request.contexts.length <= 128);
  assert.equal(core[profileIndex].id, profile.id);
  assert.equal(core[profileIndex].observations.length, request.contexts.length);
  for (const [index, context] of request.contexts.entries()) {
    for (const [phaseIndex, phase] of ['before', 'after'].entries()) {
      const actual = evaluate({ config: request[phase], context });
      assert.deepEqual(actual.errors, [], 'actual JS resolver reported an error');
      assert.equal(digest(relevant(actual.value, request.fields)), profile.js_reference_hashes[index][phaseIndex], `${profile.id}/${index}/${phase}/JS`);
      const rust = core[profileIndex].observations[index][phaseIndex];
      assert.equal(digest(relevant(rust, request.fields)), profile.rust_reference_hashes[index][phaseIndex], `${profile.id}/${index}/${phase}/Rust`);
      jsChecks++; rustChecks++;
    }
  }
  // Independent, literal expectations; these are not inferred from planner hashes.
  const context = { connector: 'a', payment_method: 'Card', payment_method_type: 'Debit' };
  const before = evaluate({ config: request.before, context }).value;
  const after = evaluate({ config: request.after, context }).value;
  assert.equal(before['dynamic_fields.choice._is_required'], true);
  assert.equal(after['dynamic_fields.choice._is_required'], true);
  assert.deepEqual(before['dynamic_fields.choice._dropdown_options'], profile.id === 'inherited-options' ? ['B', 'A'] : profile.id === 'mandate-inheritance' ? ['A'] : ['A', 'B']);
  assert.deepEqual(after['dynamic_fields.choice._dropdown_options'], profile.id === 'mandate-inheritance' ? ['A'] : ['A', 'B']);
  if (profile.id === 'mandate-inheritance') {
    const mandate = { ...context, mandate_type: 'mandate' };
    assert.deepEqual(evaluate({ config: request.before, context: mandate }).value['dynamic_fields.choice._dropdown_options'], ['A', 'B']);
    assert.deepEqual(evaluate({ config: request.after, context: mandate }).value['dynamic_fields.choice._dropdown_options'], ['A']);
  }
  const missing = evaluate({ config: request.before, context: {} }).value;
  assert.equal(missing['dynamic_fields.choice._is_required'], profile.id === 'missing-default' ? undefined : false);
}
const result = { scope: 'synthetic_fixture', profiles: cases.profiles.length, js_checks: jsChecks,
  rust_checks: rustChecks, browser: 'not_executed', server_certification: 'pending', trusted_receipt: 'not_issued' };
fs.writeFileSync(path.join(root, 'resolver-check.json'), JSON.stringify(result, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify(result));
