// Shared by scripts/demo.mjs and scripts/serve.mjs: Hyperswitch setup for the local dummy-connector stack
// (merchant, API key, fauxpay + stripe_test connectors, GSM retry rule, profile auto-retry) and small helpers.
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const HYPERSWITCH_URL = process.env.HYPERSWITCH_URL ?? 'http://localhost:8080';
const ADMIN = { Authorization: 'admin-api-key=test_admin' }; // upstream's local default admin key (dummy stack only)

// Demo amounts are USD minor units (cents). 10000 = $100.00.
export const AMOUNT = 10000;
export const CARD_OK = '4242424242424242'; // succeeds on every dummy connector
export const CARD_FLAKY = '4000000000009995'; // fails on fauxpay (DC_08, retryable), succeeds on stripe_test

export async function call(base, path, { method = 'GET', headers = {}, body } = {}) {
  const res = await fetch(base + path, {
    method,
    headers: { 'content-type': 'application/json', ...headers },
    ...(body !== undefined && { body: JSON.stringify(body) }),
  });
  const text = await res.text();
  let json;
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    json = { raw: text };
  }
  return { status: res.status, json };
}

/** Hyperswitch's /health text, or null when it is not reachable. */
export async function hyperswitchHealth(h = HYPERSWITCH_URL) {
  try {
    const res = await fetch(`${h}/health`);
    return res.ok ? await res.text() : null;
  } catch {
    return null;
  }
}

/** Creates a fresh merchant with an API key, the two dummy connectors and the GSM retry rule. */
export async function setupMerchant(h = HYPERSWITCH_URL, prefix = 'merchant_demo') {
  const mid = `${prefix}_${Date.now()}`;
  const m = await call(h, '/accounts', {
    method: 'POST',
    headers: ADMIN,
    body: { merchant_id: mid, merchant_name: 'CostRouter Demo', primary_business_details: [{ country: 'US', business: 'default' }] },
  });
  if (m.status !== 200) throw new Error(`merchant create failed: ${m.status} ${JSON.stringify(m.json)}`);
  const profile = m.json.default_profile;
  const k = await call(h, `/api_keys/${mid}`, { method: 'POST', headers: ADMIN, body: { name: 'demo', description: 'demo', expiration: 'never' } });
  if (!k.json.api_key) throw new Error(`api key create failed: ${k.status} ${JSON.stringify(k.json)}`);
  const apiKey = k.json.api_key;
  const pm = [
    {
      payment_method: 'card',
      payment_method_types: [
        { payment_method_type: 'credit', card_networks: ['Visa', 'Mastercard'], minimum_amount: 0, maximum_amount: 99999999, recurring_enabled: true, installment_payment_enabled: true },
      ],
    },
  ];
  const mca = async (name) => {
    const r = await call(h, `/account/${mid}/connectors`, {
      method: 'POST',
      headers: { 'api-key': apiKey },
      body: { connector_type: 'payment_processor', connector_name: name, connector_account_details: { auth_type: 'HeaderKey', api_key: 'dummy' }, test_mode: true, disabled: false, payment_methods_enabled: pm },
    });
    if (!r.json.merchant_connector_id) throw new Error(`connector ${name} create failed: ${r.status} ${JSON.stringify(r.json)}`);
    return r.json.merchant_connector_id;
  };
  const ids = { fauxpay: await mca('fauxpay'), stripe_test: await mca('stripe_test') };
  // GSM rule: makes fauxpay's DC_08 error retryable. It is a global row: a duplicate on re-run is harmless.
  const gsm = await call(h, '/gsm', {
    method: 'POST',
    headers: ADMIN,
    body: {
      connector: 'fauxpay', flow: 'Payment', sub_flow: 'Authorize', code: 'DC_08',
      message: 'Payment declined: Internal Server Error from Connector, Please try again later',
      status: 'failure', decision: 'retry', step_up_possible: false, clear_pan_possible: false, feature: 'retry',
      feature_data: { retry: { step_up_possible: false, clear_pan_possible: false, alternate_network_possible: false, decision: 'retry' } },
    },
  });
  return { mid, profile, apiKey, ids, gsmStatus: gsm.status };
}

export const setAutoRetry = (s, enabled, h = HYPERSWITCH_URL) =>
  call(h, `/account/${s.mid}/business_profile/${s.profile}`, {
    method: 'POST',
    headers: { 'api-key': s.apiKey },
    body: enabled ? { is_auto_retries_enabled: true, max_auto_retries_enabled: 2 } : { is_auto_retries_enabled: false },
  });

/** A Hyperswitch create-payment body for the dummy stack. */
export const payBody = (profile, card, amount = AMOUNT) => ({
  amount,
  currency: 'USD',
  confirm: true,
  capture_method: 'automatic',
  authentication_type: 'no_three_ds',
  profile_id: profile,
  payment_method: 'card',
  payment_method_type: 'credit',
  payment_method_data: { card: { card_number: card, card_exp_month: '12', card_exp_year: '2030', card_holder_name: 'Demo', card_cvc: '123' } },
  billing: { address: { line1: '1 Main St', city: 'San Francisco', state: 'CA', zip: '94000', country: 'US', first_name: 'A', last_name: 'B' } },
});

/** Builds cost-router if dist/ is missing or older than what the scripts need. */
export function ensureBuilt(root) {
  if (existsSync(join(root, 'dist', 'main.js')) && existsSync(join(root, 'dist', 'dev', 'demo-config.js'))) return;
  console.log('building cost-router (npm run build) ...');
  const r = spawnSync('npm', ['run', 'build'], { cwd: root, stdio: 'inherit', shell: process.platform === 'win32' });
  if (r.status !== 0) throw new Error('npm run build failed');
}

/** The compiled, unit-tested demo config module (src/dev/demo-config.ts): buildDemoConfig and DEMO_* values. Call ensureBuilt first. */
export async function loadDemoConfig(root) {
  const mod = await import(pathToFileURL(join(root, 'dist', 'dev', 'demo-config.js')).href);
  return mod.buildDemoConfig ? mod : mod.default;
}
