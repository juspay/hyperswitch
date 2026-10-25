#!/usr/bin/env node
/**
 * provision-merchants.mjs — one-time/occasional admin job that creates N
 * merchants (account + API key + Stripe connector) via the Hyperswitch admin
 * API and writes their credentials to a manifest (merchants.json) that
 * scenario-mix.js's `merchant_pool` config consumes.
 *
 * This is orchestration/setup work, not traffic — it is deliberately a
 * separate, plain Node.js script rather than a k6 script, so merchant
 * creation is never part of the measured load, and so progress can be
 * persisted to disk incrementally (crash-safe, resumable) instead of only at
 * the very end the way k6's handleSummary would force.
 *
 * Usage:
 *   cp provision-config.example.json provision-config.json   # fill in admin_api_key
 *   node provision-merchants.mjs                 # create/top-up up to merchant_count
 *   node provision-merchants.mjs --mode=cleanup   # delete every merchant in the manifest
 * or:
 *   PROVISION_CONFIG=/path/to/provision-config.json node provision-merchants.mjs
 *
 * Rerunning in provision mode is safe: merchant IDs are deterministic
 * (`${prefix}_0001` ...), entries already present in the manifest are
 * skipped, and a merchant_id that already exists on the server (partial
 * prior run) is recovered via GET instead of failing.
 */
import { existsSync, readFileSync } from "node:fs";
import { writeFile, rename } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { FixtureRecorder, recordedFetch } from "./sqlite.mjs";
import { progress } from "./progress.mjs";

// Defaults for connector shape; authentication must come from configuration.
const DEFAULT_CONNECTOR = {
  connector_type: "payment_processor",
  connector_name: "stripe",
  connector_account_details: {
    auth_type: "HeaderKey",
    api_key: "",
  },
  test_mode: false,
  disabled: false,
  payment_methods_enabled: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: ["Visa", "Mastercard"],
          minimum_amount: 1,
          maximum_amount: 68607706,
          recurring_enabled: true,
          installment_payment_enabled: true,
        },
        {
          payment_method_type: "debit",
          card_networks: ["Visa", "Mastercard"],
          minimum_amount: 1,
          maximum_amount: 68607706,
          recurring_enabled: true,
          installment_payment_enabled: true,
        },
      ],
    },
    {
      payment_method: "pay_later",
      payment_method_types: [
        {
          payment_method_type: "klarna",
          payment_experience: "redirect_to_url",
          minimum_amount: 1,
          maximum_amount: 68607706,
          recurring_enabled: true,
          installment_payment_enabled: true,
        },
        {
          payment_method_type: "affirm",
          payment_experience: "redirect_to_url",
          minimum_amount: 1,
          maximum_amount: 68607706,
          recurring_enabled: true,
          installment_payment_enabled: true,
        },
        {
          payment_method_type: "afterpay_clearpay",
          payment_experience: "redirect_to_url",
          minimum_amount: 1,
          maximum_amount: 68607706,
          recurring_enabled: true,
          installment_payment_enabled: true,
        },
      ],
    },
  ],
  metadata: {
    city: "NY",
    unit: "245",
  },
  business_country: "US",
  business_label: "default",
};

let activeRecorder = null;
export function setFixtureRecorder(recorder) { activeRecorder = recorder; }
async function fixtureFetch(url, options = {}) {
  const match = /\/(?:accounts|account|api_keys)\/([^/?]+)/.exec(url);
  let body = {};
  try { body = JSON.parse(options.body || "{}"); } catch (_) { /* optional body */ }
  const merchantId = match?.[1] || body.merchant_id || null;
  const operation = `${options.method || "GET"}_${new URL(url).pathname.replaceAll("/", "_")}`;
  return recordedFetch(activeRecorder,url,options,{ flow_id: `${activeRecorder?.runId || "fixture"}:${merchantId || new URL(url).pathname}`, merchant_id: merchantId, operation, role: "fixture" });
}
function fail(message) { throw new Error(`provision-merchants: ${message}`); }

function resolvePath(p) {
  return path.resolve(process.cwd(), p);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

export function loadConfig(configPath = process.env.PROVISION_CONFIG || "./provision-config.json") {
  let raw;
  try {
    raw = readFileSync(resolvePath(configPath), "utf8");
  } catch (error) {
    fail(`cannot read config at "${configPath}" (set PROVISION_CONFIG): ${error.message}`);
  }
  let cfg;
  try {
    cfg = JSON.parse(raw,(_key,value) => {
      if (typeof value !== "string") return value;
      const reference = /^\$\{([A-Za-z_][A-Za-z0-9_]*)\}$/.exec(value);
      if (!reference) return value;
      if (!process.env[reference[1]]) throw new Error(`Environment variable ${reference[1]} is required`);
      return process.env[reference[1]];
    });
  } catch (error) {
    fail(`config at "${configPath}" is not valid JSON: ${error.message}`);
  }
  if (!cfg.router) fail("router is required");
  if (!cfg.admin_api_key) fail("admin_api_key is required");
  const merchantCount = Number(cfg.merchant_count);
  if (!Number.isSafeInteger(merchantCount) || merchantCount <= 0) fail("merchant_count must be a number > 0");
  const prefix = cfg.merchant_id_prefix || "loadtest_mix";
  if (!/^[A-Za-z0-9_]+$/.test(prefix)) fail("merchant_id_prefix must use only letters, digits and underscores");
  const concurrency = cfg.concurrency === undefined ? 20 : Number(cfg.concurrency);
  if (!Number.isSafeInteger(concurrency) || concurrency < 1) fail("concurrency must be a positive integer");
  if (!cfg.connector?.connector_account_details) fail("connector.connector_account_details is required");
  const attempts = cfg.retry?.attempts ?? 3;
  const backoff = cfg.retry?.backoff_ms ?? 500;
  if (!Number.isSafeInteger(attempts) || attempts < 1 || !Number.isFinite(backoff) || backoff < 0) fail("retry attempts/backoff_ms are invalid");
  const output = cfg.output || "merchants.json";
  if (cfg.organization_id !== undefined && typeof cfg.organization_id !== "string") {
    fail("organization_id must be a string when set");
  }
  return {
    router: cfg.router.replace(/\/$/, ""),
    admin_api_key: cfg.admin_api_key,
    merchant_count: merchantCount,
    merchant_id_prefix: prefix,
    concurrency,
    output,
    // Every merchant this script creates is placed under one organization
    // (accountBody's organization_id) so the pool reads as a single tenant
    // rather than 1500 stray orgs. Leave unset to have the script create one
    // organization on first run and cache its id next to the manifest (see
    // resolveOrganizationId) — set it explicitly to reuse an org you already
    // have, or to pin the same org across manifest files/output paths.
    organization_id: cfg.organization_id || null,
    organization_name: cfg.organization_name || `${prefix} loadtest org`,
    connector: { ...DEFAULT_CONNECTOR, ...cfg.connector },
    superposition: cfg.superposition || null,
    retry: {
      attempts,
      backoff_ms: backoff,
    },
  };
}

function merchantIdFor(cfg, index) {
  const width = Math.max(4, String(cfg.merchant_count).length);
  return `${cfg.merchant_id_prefix}_${String(index + 1).padStart(width, "0")}`;
}

// ---------------------------------------------------------------------------
// Manifest (merchants.json): loaded once, rewritten atomically after every
// successful merchant so a crash never loses more than the merchant in
// flight. Writes are serialized through a promise chain — concurrent
// provisioning workers all schedule onto the same chain rather than racing
// to write the file at once.
// ---------------------------------------------------------------------------

function loadManifest(manifestPath) {
  if (!existsSync(manifestPath)) return [];
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (error) {
    fail(`cannot read existing manifest at "${manifestPath}": ${error.message}`);
  }
  if (!Array.isArray(parsed)) fail(`existing manifest at "${manifestPath}" must be a JSON array`);
  return parsed;
}

let writeChain = Promise.resolve();
function scheduleWrite(manifestPath, manifest) {
  const snapshot = structuredClone(manifest);
  writeChain = writeChain
    .then(() => writeManifestAtomic(manifestPath, snapshot));
  return writeChain;
}
async function writeManifestAtomic(manifestPath, manifest) {
  const tmpPath = `${manifestPath}.tmp`;
  await writeFile(tmpPath, JSON.stringify(manifest, null, 2), { mode: 0o600 });
  await rename(tmpPath, manifestPath);
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

function adminHeaders(cfg) {
  return { "api-key": cfg.admin_api_key, "content-type": "application/json" };
}

function merchantHeaders(apiKey) {
  return { "api-key": apiKey, "content-type": "application/json" };
}

async function safeText(res) {
  try {
    return await res.text();
  } catch (_) {
    return "";
  }
}

async function withRetry(fn, retry, label) {
  let lastError;
  for (let attempt = 1; attempt <= retry.attempts; attempt += 1) {
    try {
      return await fn();
    } catch (error) {
      if (error.recordingFatal) throw error;
      lastError = error;
      if (attempt < retry.attempts) await sleep(retry.backoff_ms * attempt);
    }
  }
  throw new Error(`${label}: ${lastError.message}`);
}

function accountBody(merchantId, orgId) {
  return {
    merchant_id: merchantId,
    locker_id: "m0010",
    merchant_name: `Loadtest Merchant ${merchantId}`,
    merchant_details: {
      primary_contact_person: "John Test",
      primary_email: "JohnTest@test.com",
      primary_phone: "sunt laborum",
      secondary_contact_person: "John Test2",
      secondary_email: "JohnTest2@test.com",
      secondary_phone: "cillum do dolor id",
      website: "www.example.com",
      about_business: "Online Retail with a wide selection of organic products for North America",
      address: {
        line1: "1467",
        line2: "Harrison Street",
        line3: "Harrison Street",
        city: "San Fransico",
        state: "California",
        zip: "94122",
        country: "US",
      },
    },
    return_url: "https://google.com/success",
    webhook_details: {
      webhook_version: "1.0.1",
      webhook_username: "ekart_retail",
      webhook_password: "password_ekart@123",
      payment_created_enabled: true,
      payment_succeeded_enabled: true,
      payment_failed_enabled: true,
    },
    sub_merchants_enabled: false,
    metadata: {
      city: "NY",
      unit: "245",
    },
    primary_business_details: [{ country: "US", business: "default" }],
    // Omitting this makes Router create a brand-new organization per
    // merchant (MerchantAccountCreate.organization_id doc comment,
    // crates/api_models/src/admin.rs) — passing the same orgId for every
    // merchant in the batch is what keeps the whole pool under one org.
    organization_id: orgId,
  };
}

async function createOrganization(cfg) {
  const res = await fixtureFetch(`${cfg.router}/organization`, {
    method: "POST",
    headers: adminHeaders(cfg),
    body: JSON.stringify({ organization_name: cfg.organization_name }),
  });
  if (!res.ok) throw new Error(`organization create failed (${res.status}): ${await safeText(res)}`);
  return res.json();
}

// The org id must stay identical across reruns of this script (a rerun
// that minted a second org would split the pool across two orgs instead of
// growing one), so an auto-created org is cached in a sidecar file next to
// the manifest rather than only printed for the user to copy by hand.
function orgCachePath(manifestPath) {
  return `${manifestPath}.org`;
}

async function resolveOrganizationId(cfg, manifestPath) {
  if (cfg.organization_id) return cfg.organization_id;
  const cachePath = orgCachePath(manifestPath);
  if (existsSync(cachePath)) {
    const cached = JSON.parse(readFileSync(cachePath, "utf8"));
    if (cached.organization_id) return cached.organization_id;
  }
  console.log(`no organization_id configured — creating one organization ("${cfg.organization_name}") for this batch...`);
  const org = await createOrganization(cfg);
  if (typeof org.organization_id !== "string" || !org.organization_id) throw new Error("organization response missing organization_id");
  await writeFile(cachePath, JSON.stringify({ organization_id: org.organization_id }, null, 2),{ mode: 0o600 });
  console.log(`created organization ${org.organization_id} (cached at ${cachePath}) — every merchant will be created under it. Set "organization_id" in provision-config.json to pin or reuse this value explicitly.`);
  return org.organization_id;
}

// A merchant_id that already exists on the server (left over from a prior
// run that crashed between account creation and the manifest write) is
// recovered via GET rather than treated as a failure, so reruns stay safe.
async function createOrRecoverAccount(merchantId, orgId, cfg) {
  const createRes = await fixtureFetch(`${cfg.router}/accounts`, {
    method: "POST",
    headers: adminHeaders(cfg),
    body: JSON.stringify(accountBody(merchantId, orgId)),
  });
  if (createRes.ok) return createRes.json();
  const recoverRes = await fixtureFetch(`${cfg.router}/accounts/${merchantId}`, { headers: adminHeaders(cfg) });
  if (recoverRes.ok) {
    const account = await recoverRes.json();
    if (account.organization_id !== orgId) throw new Error(`recovered ${merchantId} belongs to another organization`);
    return account;
  }
  throw new Error(`account create failed (${createRes.status}) and recovery GET also failed (${recoverRes.status}): ${await safeText(createRes)}`);
}

async function createApiKey(merchantId, cfg) {
  const res = await fixtureFetch(`${cfg.router}/api_keys/${merchantId}`, {
    method: "POST",
    headers: adminHeaders(cfg),
    body: JSON.stringify({ name: "scenario-mix loadtest", expiration: "2069-09-23T01:02:03.000Z" }),
  });
  if (!res.ok) throw new Error(`api_key create failed (${res.status}): ${await safeText(res)}`);
  return res.json();
}

// Unlike account create/delete (admin auth), connector create AND list are
// merchant-scoped auth (ApiKeyAuthWithMerchantIdFromRouteAllowPlatform —
// crates/router/src/routes/admin.rs, connector_create/connector_list) and
// check the api-key header against that merchant's own api_keys table
// entry, not the admin key. Must use the key this merchant's createApiKey()
// just minted.
async function findExistingConnector(merchantId, apiKey, cfg) {
  const label = `${cfg.connector.connector_name}_${cfg.connector.business_country}_${cfg.connector.business_label}`;
  const res = await fixtureFetch(`${cfg.router}/account/${merchantId}/connectors`, { headers: merchantHeaders(apiKey) });
  if (!res.ok) throw new Error(`connector list failed (${res.status}): ${await safeText(res)}`);
  const list = await res.json();
  const match = Array.isArray(list) ? list.find((c) => c.connector_label === label) : null;
  if (!match) throw new Error(`connector list has no "${label}" entry to recover`);
  return match;
}

// A merchant recovered via GET in createOrRecoverAccount (left over from an
// earlier partial run) may already have this connector attached — Router
// rejects a second create for the same (profile_id, connector_label) with a
// 400 rather than upserting. Recover the existing merchant_connector_id
// instead of failing, same spirit as createOrRecoverAccount above.
async function createConnector(merchantId, apiKey, cfg) {
  const res = await fixtureFetch(`${cfg.router}/account/${merchantId}/connectors`, {
    method: "POST",
    headers: merchantHeaders(apiKey),
    body: JSON.stringify({ ...cfg.connector, profile_id: cfg.checkpoints[merchantId].profile_id }),
  });
  if (res.ok) return res.json();
  const body = await safeText(res);
  if (res.status === 400 && body.includes("already exists")) {
    return findExistingConnector(merchantId, apiKey, cfg);
  }
  throw new Error(`connector create failed (${res.status}): ${body}`);
}

async function deleteAccount(merchantId, cfg) {
  const res = await fixtureFetch(`${cfg.router}/accounts/${merchantId}`, { method: "DELETE", headers: adminHeaders(cfg) });
  if (!res.ok) throw new Error(`delete failed (${res.status}): ${await safeText(res)}`);
}

// ---------------------------------------------------------------------------
// Concurrency: a fixed-size worker pool pulling from a shared cursor. Each
// merchant's own 3 calls (account -> api_key -> connector) stay sequential
// since each depends on the last; merchants themselves run in parallel.
// ---------------------------------------------------------------------------

async function mapLimit(items, limit, worker) {
  let cursor = 0;
  async function run() {
    while (cursor < items.length) {
      const index = cursor;
      cursor += 1;
      await worker(items[index], index);
    }
  }
  const results = await Promise.allSettled(Array.from({ length: Math.min(limit, items.length) }, run));
  const failure = results.find((result) => result.status === "rejected");
  if (failure) throw failure.reason;
}

// ---------------------------------------------------------------------------
// Provision mode
// ---------------------------------------------------------------------------

async function provisionOne(index, cfg, existing, orgId) {
  const merchantId = merchantIdFor(cfg, index);
  if (existing.has(merchantId)) return { status: "skipped", merchantId };
  try {
    const account = await withRetry(() => createOrRecoverAccount(merchantId, orgId, cfg), cfg.retry, `account ${merchantId}`);
    const profileId = account.default_profile;
    const publishableKey = account.publishable_key;
    if (!profileId || !publishableKey) {
      throw new Error(`account response missing default_profile/publishable_key: ${JSON.stringify(account)}`);
    }

    const cached = cfg.checkpoints[merchantId];
    const apiKeyResp = cached?.api_key ? cached : await withRetry(() => createApiKey(merchantId, cfg), cfg.retry, `api_key ${merchantId}`);
    if (!apiKeyResp.api_key) throw new Error(`api_key response missing api_key: ${JSON.stringify(apiKeyResp)}`);

    cfg.checkpoints[merchantId] = { api_key: apiKeyResp.api_key, profile_id: profileId, publishable_key: publishableKey };
    await scheduleWrite(cfg.checkpointPath, cfg.checkpoints);
    const connectorResp = await withRetry(() => createConnector(merchantId, apiKeyResp.api_key, cfg), cfg.retry, `connector ${merchantId}`);
    if (!connectorResp.merchant_connector_id) {
      throw new Error(`connector response missing merchant_connector_id: ${JSON.stringify(connectorResp)}`);
    }

    return {
      status: "created",
      merchantId,
      entry: {
        merchant_id: merchantId,
        api_key: apiKeyResp.api_key,
        publishable_key: publishableKey,
        profile_id: profileId,
        merchant_connector_id: connectorResp.merchant_connector_id,
      },
    };
  } catch (error) {
    if (error.recordingFatal) throw error;
    return { status: "failed", merchantId, error: error.message };
  }
}

export async function provision(cfg) {
  cfg.checkpointPath = `${resolvePath(cfg.output)}.checkpoints`;
  cfg.checkpoints = existsSync(cfg.checkpointPath) ? JSON.parse(readFileSync(cfg.checkpointPath,"utf8")) : {};
  const manifestPath = resolvePath(cfg.output);
  const manifest = loadManifest(manifestPath);
  const existing = new Map(manifest.map((entry) => [entry.merchant_id, entry]));
  const expectedIds = new Set(Array.from({ length: cfg.merchant_count },(_,index) => merchantIdFor(cfg,index)));
  if (existing.size !== manifest.length || manifest.some((entry) => !expectedIds.has(entry.merchant_id) || !entry.api_key || !entry.profile_id || !entry.publishable_key || !entry.merchant_connector_id)) {
    throw new Error("Existing fixture manifest has invalid, duplicate, or unexpected merchants; use a new output directory");
  }
  const failures = [];
  let createdCount = 0;
  let skippedCount = 0;

  const orgId = await resolveOrganizationId(cfg, manifestPath);
  console.log(`provisioning up to ${cfg.merchant_count} merchants under organization ${orgId} (${existing.size} already in manifest) at concurrency ${cfg.concurrency}...`);

  const indices = Array.from({ length: cfg.merchant_count }, (_, i) => i);
  const bar = progress("Merchants", cfg.merchant_count);
  let completed = 0;
  try {
    await mapLimit(indices, cfg.concurrency, async (index) => {
      try {
        const result = await provisionOne(index, cfg, existing, orgId);
        if (result.status === "skipped") {
          skippedCount += 1;
          return;
        }
        if (result.status === "created") {
          manifest.push(result.entry);
          existing.set(result.merchantId, result.entry);
          createdCount += 1;
          await scheduleWrite(manifestPath, manifest);
          return;
        }
        failures.push({ merchant_id: result.merchantId, error: result.error });
        console.error(`FAILED ${result.merchantId}: ${result.error}`);
      } finally {
        bar.update(++completed, `created=${createdCount} reused=${skippedCount} failed=${failures.length}`);
      }
    });
  } finally { bar.finish(); }

  await writeChain;
  if (failures.length) {
    await writeFile(path.join(path.dirname(manifestPath), "provision-failures.json"), JSON.stringify(failures, null, 2));
  }

  console.log(`\ndone: created=${createdCount} skipped=${skippedCount} failed=${failures.length} total_in_manifest=${manifest.length}`);
  if (failures.length) {
    throw new Error(`${failures.length} fixtures failed; rerun to resume`);
  }
  return { merchants: manifest, organization_id: orgId };
}

// ---------------------------------------------------------------------------
// Cleanup mode
// ---------------------------------------------------------------------------

async function cleanup(cfg) {
  const manifestPath = resolvePath(cfg.output);
  let manifest = loadManifest(manifestPath);
  if (!manifest.length) {
    console.log("nothing to clean up — manifest is empty or missing");
    return;
  }
  console.log(`deleting ${manifest.length} merchants at concurrency ${cfg.concurrency}...`);
  const failures = [];
  let deletedCount = 0;

  await mapLimit(manifest.slice(), cfg.concurrency, async (entry) => {
    try {
      await withRetry(() => deleteAccount(entry.merchant_id, cfg), cfg.retry, `delete ${entry.merchant_id}`);
      deletedCount += 1;
      manifest = manifest.filter((m) => m.merchant_id !== entry.merchant_id);
      await scheduleWrite(manifestPath, manifest);
      if (deletedCount % 50 === 0) console.log(`deleted ${deletedCount}...`);
    } catch (error) {
      failures.push({ merchant_id: entry.merchant_id, error: error.message });
      console.error(`FAILED to delete ${entry.merchant_id}: ${error.message}`);
    }
  });

  await writeChain;
  console.log(`\ndone: deleted=${deletedCount} failed=${failures.length} remaining_in_manifest=${manifest.length}`);
}

// ---------------------------------------------------------------------------

async function main() {
  const cfg = loadConfig();
  const mode = process.argv.includes("--mode=cleanup") ? "cleanup" : "provision";
  const recorder = new FixtureRecorder(path.join(path.dirname(resolvePath(cfg.output)),"fixtures.sqlite"),{ kind: mode });
  setFixtureRecorder(recorder);
  try {
    if (mode === "cleanup") await cleanup(cfg);
    else await provision(cfg);
    recorder.close();
  } catch (error) { recorder.close(error); throw error; }
  finally { setFixtureRecorder(null); }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => { console.error(error.stack || error.message); process.exitCode = 1; });
}
