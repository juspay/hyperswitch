import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

import { CONNECTOR_PAYMENT_METHODS, SERVICES } from "./config.js";

export { PAYMENT_METHODS, SERVICES } from "./config.js";

// .../cypress-tests/cypress/utils/specSelection -> .../cypress-tests
const PACKAGE_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../.."
);

/**
 * Resolves the spec files to hand to Cypress for a connector + service pair.
 *
 * Filtering is opt-in: every spec of the service is returned unless the
 * connector has a `CONNECTOR_PAYMENT_METHODS` entry. An unlisted or newly
 * onboarded connector therefore runs everything rather than silently running
 * nothing.
 *
 * @param {object} options
 * @param {string} options.service - Key of `SERVICES`.
 * @param {string} [options.connectorId] - Value of `CYPRESS_CONNECTOR`.
 * @returns {{ specs: string[], skipped: string[], filtered: boolean, reason: string }}
 */
export function resolveSpecs({ service, connectorId }) {
  const config = SERVICES[service];
  if (!config) {
    throw new Error(
      `Unknown Cypress service "${service}". Known services: ${Object.keys(SERVICES).join(", ")}`
    );
  }

  const { specDir, specMethods } = config;

  // Sorted to match the order Cypress would have globbed the directory in,
  // which the `0x-` prerequisite specs depend on.
  const files = fs
    .readdirSync(path.join(PACKAGE_ROOT, specDir))
    .filter((file) => file.endsWith(".cy.js"))
    .sort();

  const toSpecPath = (file) => `${specDir}/${file}`;

  const supported =
    CONNECTOR_PAYMENT_METHODS[String(connectorId).toLowerCase()];

  if (!supported) {
    return {
      specs: files.map(toSpecPath),
      skipped: [],
      filtered: false,
      reason: `no CONNECTOR_PAYMENT_METHODS entry for "${connectorId || "(unset)"}"`,
    };
  }

  const specs = [];
  const skipped = [];

  for (const file of files) {
    // No payment methods means "always run": a prerequisite, or a spec that has
    // not been tagged yet.
    const methods = specMethods[file];
    const shouldRun = !methods || methods.some((m) => supported.includes(m));

    (shouldRun ? specs : skipped).push(toSpecPath(file));
  }

  return {
    specs,
    skipped,
    filtered: true,
    reason: `connector "${connectorId}" supports ${supported.join(", ")}`,
  };
}

/**
 * Splits a resolved spec list across N shards for concurrent execution.
 *
 * Every shard gets the full, in-order `prerequisiteSpecs` prefix (each shard
 * is its own Cypress/Node process, so `globalState` — and the merchant it
 * seeds — is not shared across shards) followed by this shard's slice of the
 * remaining specs, round-robined by index so the naturally slower/heavier
 * specs (higher-numbered, generally later-added) don't all land in one shard.
 *
 * When `sensitiveSpecs` is non-empty, the *last* shard is reserved
 * exclusively for prerequisites + sensitive specs — see the doc comment on
 * `SERVICES` in `config.js` for why a sensitive spec can't just be pinned to
 * an arbitrary shard alongside regular ones. Regular specs then round-robin
 * across the remaining `shardTotal - 1` shards instead of all of them.
 *
 * @param {object} options
 * @param {string[]} options.specs - Output of `resolveSpecs().specs`.
 * @param {string[]} [options.prerequisiteSpecs] - Basenames every shard must
 *   run first, from `SERVICES[service].prerequisiteSpecs`.
 * @param {string[]} [options.sensitiveSpecs] - Basenames that mutate shared
 *   session state irreversibly, from `SERVICES[service].sensitiveSpecs`.
 * @param {number} options.shardIndex - 1-based shard number.
 * @param {number} options.shardTotal - Total shard count.
 * @returns {string[]} This shard's specs, prerequisites first.
 */
export function shardSpecs({
  specs,
  prerequisiteSpecs = [],
  sensitiveSpecs = [],
  shardIndex,
  shardTotal,
}) {
  if (!shardTotal || shardTotal <= 1) return specs;

  const isPrerequisite = (specPath) =>
    prerequisiteSpecs.includes(path.basename(specPath));
  const isSensitive = (specPath) =>
    sensitiveSpecs.includes(path.basename(specPath));

  const prerequisites = specs.filter(isPrerequisite);
  const sensitive = specs.filter(
    (specPath) => !isPrerequisite(specPath) && isSensitive(specPath)
  );
  const rest = specs.filter(
    (specPath) => !isPrerequisite(specPath) && !isSensitive(specPath)
  );

  if (sensitive.length === 0) {
    const shardRest = rest.filter((_, i) => i % shardTotal === shardIndex - 1);
    return [...prerequisites, ...shardRest];
  }

  if (shardIndex === shardTotal) {
    return [...prerequisites, ...sensitive];
  }

  const regularShardTotal = shardTotal - 1;
  const shardRest = rest.filter(
    (_, i) => i % regularShardTotal === shardIndex - 1
  );
  return [...prerequisites, ...shardRest];
}
