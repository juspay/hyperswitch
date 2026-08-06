"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { parseYaml } = require("../../lib/config");
const { buildPaymentConfirmBody, loadPhases, selectReadyFixtures } = require("../lib/router_api");
const { buildPlan } = require("../lib/scenarios");

const paths = ["non_modular", "modular"];
const scenarios = ["guest", "cit_on_session", "cit_off_session"];

for (const merchantPath of paths) {
  for (const name of scenarios) {
    test(`${merchantPath} ${name} produces a stable plan`, () => {
      const plan = buildPlan({ merchant_path: merchantPath, name });
      assert.equal(plan.id, `${merchantPath}:${name}`);
      assert.equal(plan.usesPmService, merchantPath === "modular");
      assert.equal(plan.requiresCustomer, name !== "guest");
    });
  }
}

test("invalid scenario selections fail before traffic starts", () => {
  assert.throws(() => buildPlan({ merchant_path: "unknown", name: "guest" }), /Unsupported merchant_path/);
  assert.throws(() => buildPlan({ merchant_path: "modular", name: "unknown" }), /Unsupported scenario/);
});

test("one-time confirms do not contain save-card fields", () => {
  const body = buildPaymentConfirmBody(buildPlan({ merchant_path: "non_modular", name: "guest" }), {}, null);
  assert.equal(body.setup_future_usage, undefined);
  assert.equal(body.customer_acceptance, undefined);
});

test("CIT confirms contain setup future usage and customer acceptance", () => {
  const body = buildPaymentConfirmBody(buildPlan({ merchant_path: "non_modular", name: "cit_off_session" }), {}, null);
  assert.equal(body.setup_future_usage, "off_session");
  assert.equal(body.customer_acceptance.acceptance_type, "online");
});

test("load phases ramp once and own their request count", () => {
  assert.deepEqual(loadPhases({ starting_rps: 1, target_rps: 5, step_rps: 2, hold_seconds: 3, idle_seconds: 1 }), [
    { rps: 1, holdSeconds: 3, idleSeconds: 1, requests: 3 },
    { rps: 3, holdSeconds: 3, idleSeconds: 1, requests: 9 },
    { rps: 5, holdSeconds: 3, idleSeconds: 1, requests: 15 },
  ]);
});

test("fixtures cannot leak between runs", () => {
  const data = { fixtures: [{ run_id: "one", state: "ready" }, { run_id: "two", state: "ready" }, { run_id: "one", state: "used" }] };
  assert.deepEqual(selectReadyFixtures(data, "one"), [data.fixtures[0]]);
});

test("shared YAML parser handles the automation config subset", () => {
  assert.deepEqual(parseYaml("scenario:\n  name: cit_on_session\nload:\n  target_rps: 10\n"), {
    scenario: { name: "cit_on_session" },
    load: { target_rps: 10 },
  });
});
