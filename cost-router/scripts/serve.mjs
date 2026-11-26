// One-command manual-testing mode: `npm run serve` (after `npm run up`).
// Creates a fresh merchant on the local dummy stack, writes a config for it to .serve/ (git-ignored, no API key),
// and runs cost-router in the foreground on port 3100 until Ctrl+C, so it can be called from Postman or curl.
// Options (env or flags): AUTO_RETRY=on|off or --auto-retry=on|off (default on);
//                         MIN_SUCCESS_RATE=0.85 or --min-success-rate=0.85 (default 0, i.e. no floor).
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AMOUNT, CARD_FLAKY, CARD_OK, HYPERSWITCH_URL as H, ensureBuilt, hyperswitchHealth, loadDemoConfig, setAutoRetry, setupMerchant } from './lib/setup.mjs';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const PORT = Number(process.env.COST_ROUTER_PORT ?? 3100);
const CR = `http://localhost:${PORT}`;

const flag = (name) => process.argv.find((a) => a.startsWith(`--${name}=`))?.split('=')[1];
const autoRetryRaw = (flag('auto-retry') ?? process.env.AUTO_RETRY ?? 'on').toLowerCase();
const minRaw = flag('min-success-rate') ?? process.env.MIN_SUCCESS_RATE ?? '0';
const minSuccessRate = Number(minRaw);
if (!['on', 'off'].includes(autoRetryRaw)) fail(`AUTO_RETRY must be on or off (got "${autoRetryRaw}")`);
if (!Number.isFinite(minSuccessRate) || minSuccessRate < 0 || minSuccessRate > 1) fail(`MIN_SUCCESS_RATE must be a number in [0, 1] (got "${minRaw}")`);
const autoRetry = autoRetryRaw === 'on';

function fail(message) {
  console.error(`serve: ${message}`);
  process.exit(1);
}

const health = await hyperswitchHealth(H);
if (!health) fail(`Hyperswitch is not reachable at ${H}. Run npm run up first.`);

ensureBuilt(ROOT);
const demo = await loadDemoConfig(ROOT);
const s = await setupMerchant(H, 'merchant_serve');
const retry = await setAutoRetry(s, autoRetry, H);
if (retry.status !== 200) fail(`could not set auto-retry on the profile: HTTP ${retry.status} ${JSON.stringify(retry.json)}`);

const dir = join(ROOT, '.serve');
mkdirSync(dir, { recursive: true });
const configPath = join(dir, 'config.json');
writeFileSync(configPath, JSON.stringify(demo.buildDemoConfig({ port: PORT, hyperswitchUrl: H, connectorIds: s.ids, minSuccessRate }), null, 2));
const envPath = join(dir, 'postman_environment.json');
writeFileSync(
  envPath,
  JSON.stringify(
    {
      name: 'cost-router (npm run serve)',
      values: [
        { key: 'cr_url', value: CR, enabled: true },
        { key: 'profile_id', value: s.profile, enabled: true },
      ],
    },
    null,
    2,
  ),
);

const child = spawn(process.execPath, ['dist/main.js'], {
  cwd: ROOT,
  env: { ...process.env, COST_ROUTER_CONFIG: configPath, HYPERSWITCH_API_KEY: s.apiKey, PORT: String(PORT) },
  stdio: 'inherit',
});
let exited = false;
child.on('exit', (code) => {
  exited = true;
  process.exit(code ?? 0);
});
// Ctrl+C reaches both processes; the router exits cleanly and this script follows it.
process.on('SIGINT', () => child.kill('SIGINT'));
process.on('SIGTERM', () => child.kill('SIGTERM'));

for (let i = 0; i < 40 && !exited; i++) {
  try {
    if ((await fetch(`${CR}/health`)).ok) break;
  } catch {
    /* starting */
  }
  await new Promise((r) => setTimeout(r, 250));
}
if (exited) fail(`cost-router exited during startup (is port ${PORT} already in use?)`);

const body = `{"amount":${AMOUNT},"currency":"USD","confirm":true,"capture_method":"automatic","authentication_type":"no_three_ds","profile_id":"${s.profile}","payment_method":"card","payment_method_type":"credit","payment_method_data":{"card":{"card_number":"${CARD_OK}","card_exp_month":"12","card_exp_year":"2030","card_holder_name":"Demo","card_cvc":"123"}},"billing":{"address":{"line1":"1 Main St","city":"San Francisco","state":"CA","zip":"94000","country":"US","first_name":"A","last_name":"B"}}}`;
console.log(`
==================== cost-router is serving ====================
cost-router   ${CR}
merchant_id   ${s.mid}
profile_id    ${s.profile}        <- paste into Postman variable profile_id
api_key       ${s.apiKey}   (local dummy stack only)
connectors    fauxpay=${s.ids.fauxpay}  stripe_test=${s.ids.stripe_test}
auto-retry    ${autoRetry ? 'on' : 'off'}     min_success_rate ${minSuccessRate}
test cards    ${CARD_OK} (succeeds everywhere)   ${CARD_FLAKY} (fails on fauxpay, succeeds on stripe_test)
config        ${configPath} (no API key in it)
postman env   ${envPath} (import it, or set profile_id by hand)

Try it:
  curl -s -X POST ${CR}/payments -H "content-type: application/json" -H "Idempotency-Key: pay_1" -d '${body}'
  curl -s ${CR}/payments/pay_1/routing-trace
Ctrl+C to stop.
================================================================`);
