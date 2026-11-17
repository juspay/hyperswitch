// End-to-end demo: cost-router in front of a real Hyperswitch (prebuilt image, dummy connectors).
// Creates a fresh merchant per run, starts the cost-router itself (fresh in-memory stats per scenario)
// and checks what it prints. Exit code 1 if any check fails.
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const H = process.env.HYPERSWITCH_URL ?? 'http://localhost:8080';
const CR_PORT = Number(process.env.COST_ROUTER_PORT ?? 3110);
const CR = `http://localhost:${CR_PORT}`;
const ROOT = fileURLToPath(new URL('..', import.meta.url));
const RUN = Date.now();
const ADMIN = { Authorization: 'admin-api-key=test_admin' };

// Demo amounts are USD minor units (cents). 10000 = $100.00.
const AMOUNT = 10000;
// ILLUSTRATIVE business assumptions for the demo only (see README "Configuration").
const FAILURE_PENALTY_PCT = 20;
const MIN_SAMPLE_SIZE = 3;
const FEES = { fauxpay: { bps: 100, fixed: 10 }, stripe_test: { bps: 290, fixed: 30 } }; // cheap+flaky vs dear+reliable
const CARD_OK = '4242424242424242'; // succeeds on every dummy connector
const CARD_FLAKY = '4000000000009995'; // fails on fauxpay (DC_08, retryable), succeeds on stripe_test

let failed = 0;
const check = (name, ok, detail = '') => {
  console.log(`   ${ok ? 'PASS' : 'FAIL'}  ${name}${ok ? '' : detail ? `  (${detail})` : ''}`);
  if (!ok) failed += 1;
};
const hr = (t) => console.log(`\n=== ${t}`);

async function call(base, path, { method = 'GET', headers = {}, body } = {}) {
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

// ---------- Hyperswitch setup ----------
async function setup() {
  const mid = `merchant_demo_${RUN}`;
  const m = await call(H, '/accounts', {
    method: 'POST',
    headers: ADMIN,
    body: { merchant_id: mid, merchant_name: 'CostRouter Demo', primary_business_details: [{ country: 'US', business: 'default' }] },
  });
  if (m.status !== 200) throw new Error(`merchant create failed: ${m.status} ${JSON.stringify(m.json)}`);
  const profile = m.json.default_profile;
  const k = await call(H, `/api_keys/${mid}`, { method: 'POST', headers: ADMIN, body: { name: 'demo', description: 'demo', expiration: 'never' } });
  const apiKey = k.json.api_key;
  const pm = [
    {
      payment_method: 'card',
      payment_method_types: [
        { payment_method_type: 'credit', card_networks: ['Visa', 'Mastercard'], minimum_amount: 0, maximum_amount: 99999999, recurring_enabled: true, installment_payment_enabled: true },
      ],
    },
  ];
  const mca = async (name) =>
    (
      await call(H, `/account/${mid}/connectors`, {
        method: 'POST',
        headers: { 'api-key': apiKey },
        body: { connector_type: 'payment_processor', connector_name: name, connector_account_details: { auth_type: 'HeaderKey', api_key: 'dummy' }, test_mode: true, disabled: false, payment_methods_enabled: pm },
      })
    ).json.merchant_connector_id;
  const ids = { fauxpay: await mca('fauxpay'), stripe_test: await mca('stripe_test') };
  // GSM rule: makes fauxpay's DC_08 error retryable. It is a global row: a duplicate on re-run is harmless.
  const g = await call(H, '/gsm', {
    method: 'POST',
    headers: ADMIN,
    body: {
      connector: 'fauxpay', flow: 'Payment', sub_flow: 'Authorize', code: 'DC_08',
      message: 'Payment declined: Internal Server Error from Connector, Please try again later',
      status: 'failure', decision: 'retry', step_up_possible: false, clear_pan_possible: false, feature: 'retry',
      feature_data: { retry: { step_up_possible: false, clear_pan_possible: false, alternate_network_possible: false, decision: 'retry' } },
    },
  });
  console.log(`merchant=${mid} profile=${profile}`);
  console.log(`connectors: fauxpay=${ids.fauxpay} stripe_test=${ids.stripe_test}`);
  console.log(`GSM retry rule for fauxpay DC_08: HTTP ${g.status}${g.status === 200 ? '' : ' (already present is fine)'}`);
  return { mid, profile, apiKey, ids };
}

const setAutoRetry = (s, enabled) =>
  call(H, `/account/${s.mid}/business_profile/${s.profile}`, {
    method: 'POST',
    headers: { 'api-key': s.apiKey },
    body: enabled ? { is_auto_retries_enabled: true, max_auto_retries_enabled: 2 } : { is_auto_retries_enabled: false },
  });

// ---------- cost-router process ----------
let child;
async function startRouter(s, minSuccessRate, label) {
  await stopRouter();
  const cfg = {
    port: CR_PORT,
    hyperswitch: { base_url: H },
    candidates: [
      { connector: 'fauxpay', merchant_connector_id: s.ids.fauxpay },
      { connector: 'stripe_test', merchant_connector_id: s.ids.stripe_test },
    ],
    routing: {
      fee_table: Object.fromEntries(Object.entries(FEES).map(([c, f]) => [c, [{ match: { currency: 'USD' }, bps: f.bps, fixed: f.fixed }]])),
      failure_penalty_pct: FAILURE_PENALTY_PCT,
      min_sample_size: MIN_SAMPLE_SIZE,
      window_minutes: 60,
      min_success_rate: minSuccessRate,
      issuer_decline_codes: [],
    },
  };
  const path = join(mkdtempSync(join(tmpdir(), 'cost-router-demo-')), 'config.json');
  writeFileSync(path, JSON.stringify(cfg, null, 2));
  child = spawn(process.execPath, ['dist/main.js'], {
    cwd: ROOT,
    env: { ...process.env, COST_ROUTER_CONFIG: path, HYPERSWITCH_API_KEY: s.apiKey, PORT: String(CR_PORT) },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let log = '';
  child.stdout.on('data', (d) => (log += d));
  child.stderr.on('data', (d) => (log += d));
  for (let i = 0; i < 40; i++) {
    try {
      if ((await fetch(`${CR}/health`)).ok) {
        console.log(`cost-router started (${label}; fresh in-memory stats)`);
        return;
      }
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`cost-router did not start:\n${log}`);
}
async function stopRouter() {
  if (!child) return;
  const c = child;
  child = undefined;
  await new Promise((r) => {
    c.once('exit', r);
    c.kill();
    setTimeout(r, 3000);
  });
}

// ---------- payments ----------
const payBody = (s, card) => ({
  amount: AMOUNT,
  currency: 'USD',
  confirm: true,
  capture_method: 'automatic',
  authentication_type: 'no_three_ds',
  profile_id: s.profile,
  payment_method: 'card',
  payment_method_type: 'credit',
  payment_method_data: { card: { card_number: card, card_exp_month: '12', card_exp_year: '2030', card_holder_name: 'Demo', card_cvc: '123' } },
  billing: { address: { line1: '1 Main St', city: 'San Francisco', state: 'CA', zip: '94000', country: 'US', first_name: 'A', last_name: 'B' } },
});
async function pay(s, id, card) {
  const r = await call(CR, '/payments', { method: 'POST', headers: { 'idempotency-key': id }, body: payBody(s, card) });
  if (r.status !== 200) throw new Error(`POST /payments ${id}: ${r.status} ${JSON.stringify(r.json)}`);
  return r.json;
}
const names = (o) => (o.routing.decision?.ordered ?? []).map((c) => c.connector);
function show(label, o) {
  const d = o.routing.decision;
  const ex = o.routing.execution;
  console.log(`-- ${label}`);
  console.log(`   decision : ${d?.trace?.summary ?? '(none)'}`);
  for (const c of d?.trace?.candidates ?? []) {
    console.log(
      `     ${c.connector.padEnd(12)} fee=${String(c.fee).padStart(4)}  s_hat=${Number(c.sHat.value).toFixed(3)}  score=${c.score ?? '-'}  rank=${c.rank ?? '-'}  ${c.status}${c.status === 'eligible' ? '' : ` (${c.statusReason})`}`,
    );
  }
  console.log(`   override : ${o.routing.routingOverride}   requested: ${JSON.stringify(ex?.requestedOrder)}`);
  console.log(`   executed : final=${ex?.finalConnector} status=${ex?.finalStatus} attempts(unordered)=${(ex?.attempts ?? []).map((a) => `${a.connector}:${a.status}`).join(', ')}`);
  for (const n of ex?.notes ?? []) console.log(`   note     : ${n}`);
}

async function main() {
  hr('Preflight');
  const health = await fetch(`${H}/health`).then((r) => r.text()).catch(() => null);
  if (!health) throw new Error(`Hyperswitch is not reachable at ${H}. Run scripts/up.sh first.`);
  console.log(`Hyperswitch: ${health}`);

  hr('Setup: merchant, two connectors (fauxpay cheap but flaky, stripe_test dearer but reliable), retry rule');
  const s = await setup();
  const fee = (f) => Math.round((AMOUNT * f.bps) / 10000) + f.fixed;
  console.log(`fees on a $${AMOUNT / 100} payment: fauxpay ${FEES.fauxpay.bps}bps+${FEES.fauxpay.fixed} = ${fee(FEES.fauxpay)} cents, stripe_test ${FEES.stripe_test.bps}bps+${FEES.stripe_test.fixed} = ${fee(FEES.stripe_test)} cents`);
  console.log(`demo assumptions (illustrative): failure_penalty_pct=${FAILURE_PENALTY_PCT}, min_sample_size=${MIN_SAMPLE_SIZE}`);

  // ---------------- Scenario A: auto-retry ON ----------------
  hr('Scenario A: auto-retry ENABLED (profile, max 2 retries); floor off to isolate the cost/reliability trade-off');
  console.log(`auto-retry on: HTTP ${(await setAutoRetry(s, true)).status}`);
  await startRouter(s, 0, 'min_success_rate=0');

  const p1 = await pay(s, `demo_${RUN}_a1`, CARD_OK);
  show('A1. cold start, good card: no reliability data yet, so the cheaper connector wins', p1);
  check('A1 ranks fauxpay (cheapest) first', names(p1)[0] === 'fauxpay' && names(p1)[1] === 'stripe_test', names(p1).join('>'));
  check('A1 fauxpay handled the payment', p1.routing.execution.finalConnector === 'fauxpay' && p1.routing.execution.finalIsFirstRequested === true);

  const p2 = await pay(s, `demo_${RUN}_a2`, CARD_FLAKY);
  show('A2. flaky card: fauxpay fails, Hyperswitch auto-retries on the next connector in our list', p2);
  check('A2 request was still [fauxpay, stripe_test]', names(p2)[0] === 'fauxpay');
  check('A2 payment recovered by fallback on stripe_test', p2.routing.execution.finalConnector === 'stripe_test' && p2.routing.execution.finalStatus === 'succeeded');
  const a2 = (p2.routing.execution.attempts ?? []).map((a) => `${a.connector}:${a.status}`).sort();
  check('A2 both attempts recorded (fauxpay failed, stripe_test charged)', a2.join() === 'fauxpay:failure,stripe_test:charged', a2.join());
  check('A2 trace does not claim an attempt order', p2.routing.execution.attemptOrderEstablished === false);

  const p3 = await pay(s, `demo_${RUN}_a3`, CARD_OK);
  show('A3. reliability now matters: the dearer but reliable connector wins despite the higher fee', p3);
  check('A3 ranks stripe_test first (lower expected cost than fauxpay)', names(p3)[0] === 'stripe_test' && names(p3)[1] === 'fauxpay', names(p3).join('>'));
  check('A3 stripe_test handled it', p3.routing.execution.finalConnector === 'stripe_test' && p3.routing.execution.finalIsFirstRequested === true);
  const cand = Object.fromEntries(p3.routing.decision.trace.candidates.map((c) => [c.connector, c]));
  check(
    'A3 fauxpay has the lower fee but the worse score',
    Number(cand.fauxpay.fee) < Number(cand.stripe_test.fee) && Number(cand.fauxpay.score) > Number(cand.stripe_test.score),
    `fee ${cand.fauxpay.fee} vs ${cand.stripe_test.fee}, score ${cand.fauxpay.score} vs ${cand.stripe_test.score}`,
  );

  hr('Routing trace endpoint: GET /payments/{id}/routing-trace');
  const tr = await call(CR, `/payments/demo_${RUN}_a3/routing-trace`);
  check('GET routing-trace returns the stored decision and execution', tr.status === 200 && tr.json.decision?.ordered?.[0]?.connector === 'stripe_test' && tr.json.execution?.finalConnector === 'stripe_test');
  console.log(
    JSON.stringify(
      {
        paymentId: tr.json.paymentId,
        routingOverride: tr.json.routingOverride,
        ordered: names({ routing: tr.json }),
        summary: tr.json.decision?.trace?.summary,
        failurePenaltyPct: tr.json.decision?.trace?.failurePenaltyPct,
        lEff: tr.json.decision?.trace?.lEff,
        breakEvenPenaltyPct: tr.json.decision?.trace?.breakEvenPenaltyPct,
        execution: tr.json.execution,
      },
      null,
      2,
    ),
  );
  const v1 = await call(CR, `/v1/payments/demo_${RUN}_a3/routing-trace`);
  check('GET /v1/payments/{id}/routing-trace (the path named in the assignment) returns the same record', v1.status === 200 && JSON.stringify(v1.json) === JSON.stringify(tr.json));
  check('unknown payment id returns 404', (await call(CR, '/payments/does_not_exist/routing-trace')).status === 404);

  hr('Idempotent replay');
  const rep = await call(CR, '/payments', { method: 'POST', headers: { 'idempotency-key': `demo_${RUN}_a3` }, body: payBody(s, CARD_OK) });
  check('same key + same body returns the stored result (replayed:true)', rep.status === 200 && rep.json.replayed === true);
  const conflict = await call(CR, '/payments', { method: 'POST', headers: { 'idempotency-key': `demo_${RUN}_a3` }, body: { ...payBody(s, CARD_OK), amount: AMOUNT + 1 } });
  check('same key + different body is rejected with 409', conflict.status === 409, String(conflict.status));

  // ---------------- Scenario B: auto-retry OFF + reliability floor ----------------
  hr('Scenario B: auto-retry DISABLED, reliability floor min_success_rate=0.85 (illustrative assumption)');
  console.log(`auto-retry off: HTTP ${(await setAutoRetry(s, false)).status}`);
  await startRouter(s, 0.85, 'min_success_rate=0.85');
  const b1 = await pay(s, `demo_${RUN}_b1`, CARD_FLAKY);
  show('B1. flaky card, retries off: fauxpay fails and there is NO failover even though stripe_test was in our list', b1);
  check('B1 request was [fauxpay, stripe_test]', names(b1)[0] === 'fauxpay');
  const b1n = (b1.routing.execution.attempts ?? []).length;
  check('B1 payment failed after a single attempt', b1.routing.execution.finalStatus === 'failed' && b1n === 1, `${b1.routing.execution.finalStatus} attempts=${b1n}`);

  const b2 = await pay(s, `demo_${RUN}_b2`, CARD_FLAKY);
  show('B2. every connector is now below the floor: the layer steps aside and Hyperswitch routes normally', b2);
  check(
    'B2 reliability_floor_unmet, no override sent',
    b2.routing.decision.fallbackReason === 'reliability_floor_unmet' && b2.routing.routingOverride === 'omitted_fallback' && b2.routing.execution.requestedOrder === null,
    `${b2.routing.decision.fallbackReason}/${b2.routing.routingOverride}`,
  );
  const t2 = await call(CR, `/payments/demo_${RUN}_b2/routing-trace`);
  check('B2 trace explains each exclusion', t2.status === 200 && t2.json.decision.trace.candidates.every((c) => /reliability_floor_unmet/.test(c.statusReason)));

  await stopRouter();
  hr(failed === 0 ? 'ALL CHECKS PASSED' : `${failed} CHECK(S) FAILED`);
  process.exitCode = failed === 0 ? 0 : 1;
}

main().catch(async (e) => {
  console.error(`\nDEMO ERROR: ${e.message}`);
  await stopRouter();
  process.exitCode = 1;
});
