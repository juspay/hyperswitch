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
 * Usage:
 *   node scripts/run-cypress.js <service> [...extra cypress args]
 *   node scripts/run-cypress.js <service> --print-specs   # dry run
 */

import { spawnSync } from "child_process";
import path from "path";
import { fileURLToPath } from "url";

import {
  resolveSpecs,
  SERVICES,
} from "../cypress/utils/specSelection/index.js";

const PACKAGE_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  ".."
);

const [service, ...args] = process.argv.slice(2);

if (!SERVICES[service]) {
  console.error(
    `Usage: node scripts/run-cypress.js <${Object.keys(SERVICES).join("|")}> [...cypress args]`
  );
  process.exit(1);
}

const { specs, skipped, filtered, reason } = resolveSpecs({
  service,
  connectorId: process.env.CYPRESS_CONNECTOR,
});

const summary = filtered
  ? `running ${specs.length} spec(s), skipping ${skipped.length}`
  : `running all ${specs.length} spec(s)`;
console.log(`[spec-selection] ${service}: ${summary} — ${reason}`);

if (args.includes("--print-specs")) {
  console.log(specs.join("\n"));
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
