// Load test of the custom path: POST /payments through cost-router at a fixed open-loop rate.
// Hyperswitch is replaced by an in-process stub that answers instantly, so the numbers measure cost-router's
// own path (decision, forward, attempt read-back, stats, trace) and not the connectors or the stack.
// Usage: npm run build && node scripts/loadtest.mjs   (env: LOADTEST_RPS=100, LOADTEST_SECONDS=60)
// Writes docs/LOADTEST.md and exits 1 if p99 >= 200 ms or any request failed.
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

const RPS = Number(process.env.LOADTEST_RPS ?? 100);
const SECONDS = Number(process.env.LOADTEST_SECONDS ?? 60);
const PORT = Number(process.env.LOADTEST_PORT ?? 3130);
const P99_LIMIT_MS = 200;
const ROOT = fileURLToPath(new URL('..', import.meta.url));

// ---------- stub Hyperswitch ----------
const stub = createServer((req, res) => {
  let raw = '';
  req.on('data', (c) => (raw += c));
  req.on('end', () => {
    res.setHeader('content-type', 'application/json');
    if (req.method === 'POST') {
      const body = JSON.parse(raw || '{}');
      const connector = body.routing?.data?.[0]?.connector ?? 'fauxpay';
      res.end(JSON.stringify({ payment_id: body.payment_id, status: 'succeeded', connector }));
    } else {
      const id = decodeURIComponent((req.url ?? '').split('/')[2]?.split('?')[0] ?? '');
      res.end(JSON.stringify({ payment_id: id, status: 'succeeded', attempts: [{ attempt_id: `${id}_1`, connector: 'fauxpay', status: 'charged' }] }));
    }
  });
});
await new Promise((r) => stub.listen(0, '127.0.0.1', r));
const stubUrl = `http://127.0.0.1:${stub.address().port}`;

// ---------- cost-router ----------
const cfgPath = join(mkdtempSync(join(tmpdir(), 'cost-router-load-')), 'config.json');
writeFileSync(
  cfgPath,
  JSON.stringify({
    port: PORT,
    hyperswitch: { base_url: stubUrl },
    candidates: [
      { connector: 'fauxpay', merchant_connector_id: 'mca_fauxpay' },
      { connector: 'stripe_test', merchant_connector_id: 'mca_stripe_test' },
    ],
    routing: {
      fee_table: {
        fauxpay: [{ match: { currency: 'USD' }, bps: 100, fixed: 10 }],
        stripe_test: [{ match: { currency: 'USD' }, bps: 290, fixed: 30 }],
      },
      failure_penalty_pct: 20,
      min_sample_size: 3,
      window_minutes: 60,
      min_success_rate: 0,
    },
  }),
);
const child = spawn(process.execPath, ['dist/main.js'], {
  cwd: ROOT,
  env: { ...process.env, COST_ROUTER_CONFIG: cfgPath, HYPERSWITCH_API_KEY: 'loadtest', PORT: String(PORT), COST_ROUTER_STATE_FILE: '' },
  stdio: 'ignore',
});
const CR = `http://127.0.0.1:${PORT}`;
for (let i = 0; ; i++) {
  try {
    if ((await fetch(`${CR}/health`)).ok) break;
  } catch {
    /* starting */
  }
  if (i > 40) throw new Error('cost-router did not start (run npm run build first)');
  await new Promise((r) => setTimeout(r, 250));
}

// ---------- open-loop load: request i is due at start + i/RPS, whether or not earlier ones finished ----------
const total = RPS * SECONDS;
const latencies = [];
let errors = 0;
const statusCounts = {};
const inflight = [];
const send = async (i) => {
  const t = performance.now();
  try {
    const res = await fetch(`${CR}/payments`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'idempotency-key': `load_${i}` },
      body: JSON.stringify({ amount: 10000, currency: 'USD', confirm: true }),
    });
    await res.arrayBuffer();
    statusCounts[res.status] = (statusCounts[res.status] ?? 0) + 1;
    if (res.status !== 200) errors += 1;
  } catch {
    errors += 1;
    statusCounts.network_error = (statusCounts.network_error ?? 0) + 1;
  }
  latencies.push(performance.now() - t);
};
const start = performance.now();
let next = 0;
while (next < total) {
  const due = Math.min(total, Math.floor(((performance.now() - start) / 1000) * RPS) + 1);
  for (; next < due; next++) inflight.push(send(next));
  await new Promise((r) => setTimeout(r, 2));
}
const sendWindowS = (performance.now() - start) / 1000;
await Promise.all(inflight);

const metrics = await (await fetch(`${CR}/metrics`)).text();
child.kill();
stub.close();

latencies.sort((a, b) => a - b);
const pct = (p) => latencies[Math.min(latencies.length - 1, Math.ceil((p / 100) * latencies.length) - 1)];
const r = {
  targetRps: RPS,
  durationS: SECONDS,
  requests: latencies.length,
  achievedRps: +(latencies.length / sendWindowS).toFixed(1),
  errors,
  statusCounts,
  p50: +pct(50).toFixed(1),
  p90: +pct(90).toFixed(1),
  p99: +pct(99).toFixed(1),
  max: +latencies[latencies.length - 1].toFixed(1),
  decisionsCounted: Number(/cost_router_routing_decisions_total\{[^}]*\} (\d+)/.exec(metrics)?.[1] ?? NaN),
};
const pass = r.errors === 0 && r.p99 < P99_LIMIT_MS;
console.log(JSON.stringify(r, null, 2));
console.log(pass ? `PASS: p99 ${r.p99} ms < ${P99_LIMIT_MS} ms with no errors` : `FAIL: p99 ${r.p99} ms, errors ${r.errors}`);

writeFileSync(
  join(ROOT, 'docs', 'LOADTEST.md'),
  `# Load test: ${RPS} RPS for ${SECONDS} s on the custom path

Command: \`npm run build && npm run loadtest\` (\`scripts/loadtest.mjs\`). Run on ${new Date().toISOString().slice(0, 10)}, Windows 10, 8 GB RAM, Node ${process.version}.

**What is measured:** \`POST /payments\` through cost-router at an open-loop ${RPS} requests/second (each request is sent on schedule,
whether or not earlier ones finished), each with a new Idempotency-Key. Every request runs the full custom path: routing decision,
forward to Hyperswitch, attempt read-back (\`GET ...?expand_attempts=true\`), stats update, trace storage.

**What is not measured:** Hyperswitch and the connectors. They are replaced by an in-process stub that answers immediately, so this is
cost-router's own latency. Against the real stack, Hyperswitch's and the PSP's time is added on top.

| Metric | Value |
|---|---|
| Requests | ${r.requests} (${r.achievedRps} RPS achieved) |
| Errors | ${r.errors} (${JSON.stringify(r.statusCounts)}) |
| p50 / p90 / **p99** / max | ${r.p50} / ${r.p90} / **${r.p99}** / ${r.max} ms |
| Decisions counted on \`/metrics\` | ${r.decisionsCounted} |
| Result | **${pass ? 'PASS' : 'FAIL'}** (target p99 < ${P99_LIMIT_MS} ms) |
`,
);
process.exit(pass ? 0 : 1);
