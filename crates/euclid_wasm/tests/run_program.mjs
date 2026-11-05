import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const wasmDirectory = process.env.EUCLID_WASM_DIR
  ? path.resolve(process.env.EUCLID_WASM_DIR)
  : fileURLToPath(new URL("../../../wasm/", import.meta.url));
const {
  default: init,
  analyzeProgram,
  getAllKeys,
  runProgram,
} = await import(pathToFileURL(path.join(wasmDirectory, "euclid.js")));
await init({
  module_or_path: await readFile(path.join(wasmDirectory, "euclid_bg.wasm")),
});

const selection = (connector) => ({
  type: "priority",
  data: [{ connector, merchant_connector_id: undefined }],
});

function input() {
  return {
    payment: {
      amount: 1000,
      currency: "USD",
      billing_country: "UnitedStatesOfAmerica",
      business_country: "UnitedStatesOfAmerica",
      card_bin: "424242",
    },
    payment_method: {
      payment_method: "card",
      payment_method_type: "credit",
      card_network: "Visa",
    },
    mandate: { mandate_type: "single_use", payment_type: "non_mandate" },
    metadata: { segment: "vip" },
  };
}

function program(key, value) {
  return {
    defaultSelection: selection("adyen"),
    rules: [
      {
        name: "matching_rule",
        connectorSelection: selection("stripe"),
        statements: [
          {
            condition: [
              ...(key === "card_type"
                ? [
                    {
                      lhs: "payment_method",
                      comparison: "equal",
                      value: { type: "enum_variant", value: "card" },
                      metadata: {},
                    },
                  ]
                : []),
              { lhs: key, comparison: "equal", value, metadata: {} },
            ],
            nested: null,
          },
        ],
      },
    ],
    metadata: {},
  };
}

const cases = [
  {
    key: "card_type",
    value: { type: "enum_variant", value: "credit" },
    section: "payment_method",
    field: "payment_method_type",
    other: "debit",
  },
  {
    key: "billing_country",
    value: { type: "enum_variant", value: "UnitedStatesOfAmerica" },
    section: "payment",
    field: "billing_country",
    other: "India",
  },
  {
    key: "business_country",
    value: { type: "enum_variant", value: "UnitedStatesOfAmerica" },
    section: "payment",
    field: "business_country",
    other: "India",
  },
  {
    key: "card_network",
    value: { type: "enum_variant", value: "Visa" },
    section: "payment_method",
    field: "card_network",
    other: "Mastercard",
  },
  {
    key: "card_bin",
    value: { type: "str_value", value: "424242" },
    section: "payment",
    field: "card_bin",
    other: "555555",
  },
  {
    key: "mandate_type",
    value: { type: "enum_variant", value: "single_use" },
    section: "mandate",
    field: "mandate_type",
    other: "multi_use",
  },
  {
    key: "metadata",
    value: {
      type: "metadata_variant",
      value: { key: "segment", value: "vip" },
    },
    section: "metadata",
    field: "segment",
    other: "standard",
  },
];

for (const { key, value, section, field, other } of cases) {
  test(`${key} selects the matching rule`, () => {
    assert(getAllKeys().includes(key));
    const rule = program(key, value);
    analyzeProgram(rule);
    assert.deepEqual(runProgram(rule, input()), {
      rule_name: "matching_rule",
      connector_selection: selection("stripe"),
    });
  });

  test(`${key} uses the default for a nonmatching value`, () => {
    const payment = input();
    payment[section][field] = other;
    assert.deepEqual(runProgram(program(key, value), payment), {
      rule_name: undefined,
      connector_selection: selection("adyen"),
    });
  });

  test(`${key} uses the default when the input field is absent`, () => {
    const payment = input();
    delete payment[section][field];
    assert.deepEqual(runProgram(program(key, value), payment), {
      rule_name: undefined,
      connector_selection: selection("adyen"),
    });
  });
}

for (const [key, value] of [
  ["currency", { type: "enum_variant", value: "USD" }],
  ["amount", { type: "number", value: 1000 }],
  ["payment_method", { type: "enum_variant", value: "card" }],
]) {
  test(`${key} still selects the matching rule`, () => {
    const rule = program(key, value);
    analyzeProgram(rule);
    assert.deepEqual(runProgram(rule, input()), {
      rule_name: "matching_rule",
      connector_selection: selection("stripe"),
    });
  });
}

for (const [value, connector] of [
  ["card", "adyen"],
  ["wallet", "stripe"],
  [undefined, "stripe"],
]) {
  test(`payment_method not_equal follows server semantics for ${value}`, () => {
    const rule = program("payment_method", {
      type: "enum_variant",
      value: "card",
    });
    rule.rules[0].statements[0].condition[0].comparison = "not_equal";
    analyzeProgram(rule);
    const payment = input();
    payment.payment_method.payment_method = value;
    assert.deepEqual(runProgram(rule, payment), {
      rule_name: connector === "stripe" ? "matching_rule" : undefined,
      connector_selection: selection(connector),
    });
  });
}

for (const value of [
  { type: "number", value: 1000 },
  { type: "number_array", value: [1000, 2000] },
]) {
  for (const amount of [1000, 2000, 3000]) {
    test(`amount not_equal ${JSON.stringify(value)} evaluates ${amount}`, () => {
      const rule = program("amount", value);
      rule.rules[0].statements[0].condition[0].comparison = "not_equal";
      analyzeProgram(rule);
      const payment = input();
      payment.payment.amount = amount;
      const matches = Array.isArray(value.value)
        ? !value.value.includes(amount)
        : value.value !== amount;
      assert.deepEqual(runProgram(rule, payment), {
        rule_name: matches ? "matching_rule" : undefined,
        connector_selection: selection(matches ? "stripe" : "adyen"),
      });
    });
  }
}
