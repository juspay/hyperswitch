#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * Entry point behind every `cypress:<service>` npm script.
 *
 * Resolves the spec list for `CYPRESS_CONNECTOR` — the mandatory setup specs
 * plus only the specs whose payment methods the connector supports — and runs
 * Cypress against exactly that list. Falls back to the full spec directory when
 * the connector's payment methods cannot be determined.
 *
 * `--shard <index>/<total>` runs only this shard's slice (see `shardSpecs` in
 * `cypress/utils/specSelection/index.js`), for splitting one connector's specs
 * across concurrent Cypress processes. Not forwarded to Cypress itself.
 *
 * Usage:
 *   node scripts/run-cypress.js <service> [...extra cypress args]
 *   node scripts/run-cypress.js <service> --print-specs   # dry run
 *   node scripts/run-cypress.js <service> --shard 1/3     # this shard only
 */

import { spawnSync } from "child_process";
import path from "path";
import { fileURLToPath } from "url";

import {
  resolveSpecs,
  shardSpecs,
  SERVICES,
} from "../cypress/utils/specSelection/index.js";

const PACKAGE_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);

const [service, ...rawArgs] = process.argv.slice(2);

if (!SERVICES[service]) {
  console.error(
    `Usage: node scripts/run-cypress.js <${Object.keys(SERVICES).join("|")}> [...cypress args]`
  );
  process.exit(1);
}

let shardIndex = null;
let shardTotal = null;
const args = [];
for (let i = 0; i < rawArgs.length; i++) {
  if (rawArgs[i] === "--shard") {
    const value = rawArgs[++i];
    const match = /^(\d+)\/(\d+)$/.exec(value ?? "");
    if (!match) {
      console.error(
        `Invalid --shard value "${value}", expected "<index>/<total>" (1-based index)`
      );
      process.exit(1);
    }
    shardIndex = Number(match[1]);
    shardTotal = Number(match[2]);
    if (shardIndex < 1 || shardIndex > shardTotal) {
      console.error(
        `--shard index ${shardIndex} out of range for ${shardTotal} shard(s)`
      );
      process.exit(1);
    }
  } else {
    args.push(rawArgs[i]);
  }
}

const {
  specs: resolvedSpecs,
  skipped,
  filtered,
  reason,
} = resolveSpecs({
  service,
  connectorId: process.env.CYPRESS_CONNECTOR,
});

const specs = shardTotal
  ? shardSpecs({
      specs: resolvedSpecs,
      prerequisiteSpecs: SERVICES[service].prerequisiteSpecs,
      shardIndex,
      shardTotal,
    })
  : resolvedSpecs;

const shardLabel = shardTotal ? ` [shard ${shardIndex}/${shardTotal}]` : "";
const summary = filtered
  ? `running ${specs.length} spec(s), skipping ${skipped.length}`
  : `running all ${specs.length} spec(s)`;
console.log(`[spec-selection] ${service}${shardLabel}: ${summary} — ${reason}`);

if (args.includes("--print-specs")) {
  console.log(specs.join("\n"));
  process.exit(0);
}

const prerequisiteCount = SERVICES[service].prerequisiteSpecs?.length ?? 0;
if (shardTotal && specs.length <= prerequisiteCount) {
  console.log(
    `[spec-selection] ${service}${shardLabel}: no specs beyond the prerequisites for this shard, skipping`
  );
  process.exit(0);
}

const cypress = spawnSync(
  "npm",
  [
    "exec",
    "--",
    "cypress",
    "run",
    "--headless",
    "--spec",
    specs.join(","),
    ...args,
  ],
  {
    cwd: PACKAGE_ROOT,
    stdio: "inherit",
    shell: process.platform === "win32",
  }
);

if (cypress.error) {
  console.error(cypress.error.message);
  process.exit(1);
}

process.exit(cypress.status ?? 1);
