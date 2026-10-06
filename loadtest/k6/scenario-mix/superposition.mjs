import { isDeepStrictEqual } from "node:util";
import { recordedFetch } from "./sqlite.mjs";
import { progress } from "./progress.mjs";

function validate(config) {
  if (!config || typeof config.defaults !== "object" || !config.defaults || Array.isArray(config.defaults)) throw new Error("superposition.defaults must be a flag/value object");
  for (const key of ["endpoint", "token", "organization_id", "workspace_id"]) {
    if (typeof config[key] !== "string" || !config[key]) throw new Error(`superposition.${key} is required`);
  }
  if (!Number.isSafeInteger(config.concurrency ?? 20) || (config.concurrency ?? 20) < 1) throw new Error("superposition.concurrency must be a positive integer");
  if (!Number.isFinite(config.refresh_interval_seconds) || config.refresh_interval_seconds < 0) throw new Error("superposition.refresh_interval_seconds must specify the deployed service polling interval");
}

export async function configureSuperposition(config, recorder, merchants, organizationId) {
  if (!config) return null;
  validate(config);
  const headers = { "content-type": "application/json", Authorization: `Bearer ${config.token}`,
    "x-org-id": config.organization_id, "x-workspace": config.workspace_id };
  const base = config.endpoint.replace(/\/$/, "");
  async function call(method, route, body, merchantId = null) {
    const response = await recordedFetch(recorder,`${base}${route}`, { method, headers, ...(body === undefined ? {} : { body: JSON.stringify(body) }) },
      { operation: `superposition_${method}_${route.split("?")[0]}`, merchant_id: merchantId, role: "fixture" });
    if (!response.ok) throw new Error(`Superposition ${method} ${route}: HTTP ${response.status}: ${await response.text()}`);
    return { body: await response.json(), version: response.headers.get("x-config-version") };
  }
  // Discover every key before changing anything. PATCH only the value, preserving schema/functions.
  const current = new Map();
  for (const key of Object.keys(config.defaults)) {
    const existing = await call("GET",`/default-config/${encodeURIComponent(key)}`);
    current.set(key,existing.body.value);
  }
  for (const [key,value] of Object.entries(config.defaults)) {
    if (isDeepStrictEqual(current.get(key),value)) continue;
    await call("PATCH",`/default-config/${encodeURIComponent(key)}`,{ value, change_reason: "scenario-mix fixture defaults" });
  }
  let version = null;
  async function verify(label) {
    const bar = progress(label, merchants.length);
    let completed = 0;
    let cursor = 0;
    let failure;
    async function worker() {
      while (!failure && cursor < merchants.length) {
        const merchant = merchants[cursor++];
        try {
          const resolved = await call("POST","/config/resolve",{ context: {
            ...(config.context || {}), organization_id: organizationId, merchant_id: merchant.merchant_id, profile_id: merchant.profile_id,
          } },merchant.merchant_id);
          for (const [key,value] of Object.entries(config.defaults)) {
            if (!isDeepStrictEqual(resolved.body[key],value)) throw new Error(`Superposition override conflict for ${merchant.merchant_id}: ${key}`);
          }
          if (!resolved.version) throw new Error("Superposition response lacks x-config-version; cannot verify a stable configuration");
          if (version !== null && version !== resolved.version) throw new Error("Superposition configuration changed during fixture verification; rerun fixtures");
          version = resolved.version;
          bar.update(++completed);
        } catch (error) { failure ||= error; }
      }
    }
    try {
      await Promise.all(Array.from({ length: Math.min(config.concurrency ?? 20,merchants.length) },worker));
    } finally { bar.finish(); }
    if (failure) throw failure;
  }
  await verify("Resolve flags");
  if (config.refresh_interval_seconds > 0) console.log(`Waiting ${config.refresh_interval_seconds}s for Superposition service refresh before readiness verification...`);
  const deadline = Date.now() + config.refresh_interval_seconds * 1000;
  if (config.refresh_interval_seconds > 0) {
    const seconds = config.refresh_interval_seconds;
    const bar = progress("Service refresh", seconds);
    try {
      while (Date.now() < deadline) {
        await new Promise((resolve) => setTimeout(resolve,Math.min(1000,deadline-Date.now())));
        const remaining = Math.max(0,Math.ceil((deadline-Date.now())/1000));
        bar.update(seconds-remaining, `${remaining}s remaining`);
      }
    } finally { bar.finish(); }
  }
  await verify("Verify refreshed flags");
  return { organization_id: config.organization_id, workspace_id: config.workspace_id,
    defaults: config.defaults, version, refresh_interval_seconds: config.refresh_interval_seconds };
}
