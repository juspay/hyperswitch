import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

import {
  CONNECTOR_PAYMENT_METHODS,
  SERVICE_SPEC_DIRS,
  SERVICE_SPEC_METHODS,
} from "./config.js";

export { PAYMENT_METHODS, SERVICE_SPEC_DIRS } from "./config.js";

// .../cypress-tests/cypress/utils/specSelection -> .../cypress-tests
const PACKAGE_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../.."
);

/**
 * Resolves the spec files to hand to Cypress for a connector + service pair.
 *
 * Filtering is opt-in: every spec of the service is returned unless the service
 * is payment-method scoped *and* the connector has a `CONNECTOR_PAYMENT_METHODS`
 * entry. An unlisted or newly onboarded connector therefore runs everything
 * rather than silently running nothing.
 *
 * @param {object} options
 * @param {string} options.service - Key of `SERVICE_SPEC_DIRS`.
 * @param {string} [options.connectorId] - Value of `CYPRESS_CONNECTOR`.
 * @returns {{ specs: string[], skipped: string[], filtered: boolean, reason: string }}
 */
export function resolveSpecs({ service, connectorId }) {
  const specDir = SERVICE_SPEC_DIRS[service];
  if (!specDir) {
    throw new Error(
      `Unknown Cypress service "${service}". Known services: ${Object.keys(SERVICE_SPEC_DIRS).join(", ")}`
    );
  }

  // Sorted to match the order Cypress would have globbed the directory in,
  // which the `0x-` prerequisite specs depend on.
  const files = fs
    .readdirSync(path.join(PACKAGE_ROOT, specDir))
    .filter((file) => file.endsWith(".cy.js"))
    .sort();

  const toSpecPath = (file) => `${specDir}/${file}`;

  const specMethods = SERVICE_SPEC_METHODS[service];
  const supported =
    CONNECTOR_PAYMENT_METHODS[String(connectorId).toLowerCase()];

  if (!specMethods || !supported) {
    return {
      specs: files.map(toSpecPath),
      skipped: [],
      filtered: false,
      reason: specMethods
        ? `no CONNECTOR_PAYMENT_METHODS entry for "${connectorId || "(unset)"}"`
        : `service "${service}" is not payment-method scoped`,
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
