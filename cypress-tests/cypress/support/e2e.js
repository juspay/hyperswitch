// ***********************************************************
// This example support/e2e.js is processed and
// loaded automatically before your test files.
//
// This is a great place to put global configuration and
// behavior that modifies Cypress.
//
// You can change the location of this file or turn off
// automatically serving support files with the
// 'supportFile' configuration option.
//
// You can read more here:
// https://on.cypress.io/configuration
// ***********************************************************

// Import commands.js using ES2015 syntax:
import "cypress-mochawesome-reporter/register";
import "./commands";
import "./redirectionHandler";

Cypress.on("window:before:load", (win) => {
  // Add security headers
  win.headers = {
    "Content-Security-Policy": "default-src 'self'",
    "X-Content-Type-Options": "nosniff",
    "X-Frame-Options": "DENY",
  };
});

// Add error handling for dynamic imports
Cypress.on("uncaught:exception", (err, runnable) => {
  // Log the error details
  // eslint-disable-next-line no-console
  console.error(
    `Error: ${err.message}\nError occurred in: ${runnable.title}\nStack trace: ${err.stack}`
  );

  // Return false to prevent the error from failing the test
  return false;
});

// ─────────────────────────────────────────────────────────────────────────
// MITM proxy integration (record/replay of connector traffic)
//
// When CYPRESS_PROXY_ADMIN_URL is set, every test:
//   1. notifies the proxy of test start/end so capture mode can organise
//      cassettes by test name on disk;
//   2. tags every cy.request with X-Request-ID = <test-hash>-<step-counter>,
//      which the Hyperswitch router (with trace_header.id_reuse_strategy
//      = "use_incoming") propagates onto its connector outbound. The proxy
//      uses that ID as the cassette match key on replay.
//
// IDs are deterministic across runs of the same test, so a cassette
// recorded once replays in identical order on subsequent runs.
// ─────────────────────────────────────────────────────────────────────────
const PROXY_ADMIN_URL = Cypress.env("PROXY_ADMIN_URL");

let stepCounter = 0;
let testIdHash = "";

function djb2(str) {
  let h = 5381;
  for (let i = 0; i < str.length; i++) {
    h = ((h * 33) ^ str.charCodeAt(i)) >>> 0;
  }
  return h.toString(16).padStart(8, "0");
}

function normalizeRequestArgs(args) {
  const HTTP_METHODS = new Set([
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "HEAD",
    "OPTIONS",
    "TRACE",
    "CONNECT",
  ]);
  if (args.length === 1) {
    const a = args[0];
    if (typeof a === "string") return { url: a };
    if (a && typeof a === "object") return { ...a };
  }
  if (args.length === 2) {
    const [a, b] = args;
    if (typeof a === "string" && HTTP_METHODS.has(a.toUpperCase())) {
      return { method: a, url: b };
    }
    return { url: a, body: b };
  }
  if (args.length === 3) {
    return { method: args[0], url: args[1], body: args[2] };
  }
  return { url: args[0] };
}

beforeEach(() => {
  const titlePath = Cypress.currentTest.titlePath;
  const title = titlePath.join(" > ");
  testIdHash = djb2(title);
  stepCounter = 0;
  if (PROXY_ADMIN_URL) {
    cy.request({
      method: "POST",
      url: `${PROXY_ADMIN_URL}/test/start`,
      body: { titlePath: titlePath, spec: Cypress.spec.relative },
      failOnStatusCode: false,
      timeout: 2000,
    });
  }
});

afterEach(() => {
  if (PROXY_ADMIN_URL) {
    cy.request({
      method: "POST",
      url: `${PROXY_ADMIN_URL}/test/end`,
      failOnStatusCode: false,
      timeout: 2000,
    });
  }
});

Cypress.Commands.overwrite("request", (originalFn, ...args) => {
  const opts = normalizeRequestArgs(args);

  // Skip ID injection for proxy admin server calls (avoids noise)
  if (PROXY_ADMIN_URL && opts.url && opts.url.startsWith(PROXY_ADMIN_URL)) {
    return originalFn(opts);
  }

  stepCounter += 1;
  const requestId = `${testIdHash}-${String(stepCounter).padStart(3, "0")}`;
  opts.headers = { ...(opts.headers || {}), "X-Request-ID": requestId };
  return originalFn(opts);
});
