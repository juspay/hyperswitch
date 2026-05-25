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
// The whole integration is gated by IS_PROXY_ENABLED. When it is unset or not
// "true", none of the hooks below are registered and Cypress behaves normally.
//
// When enabled, every test:
//   1. notifies the proxy of test start/end (only when PROXY_ADMIN_URL is also
//      set) so capture mode can organise cassettes by test name on disk;
//   2. tags every cy.request with X-Request-ID = <test-hash>-<step-counter>,
//      which the Hyperswitch router (with trace_header.id_reuse_strategy
//      = "use_incoming") propagates onto its connector outbound. The proxy
//      uses that ID as the cassette match key on replay.
//
// IDs are deterministic across runs of the same test, so a cassette
// recorded once replays in identical order on subsequent runs.
// ─────────────────────────────────────────────────────────────────────────
const IS_PROXY_ENABLED = String(Cypress.env("IS_PROXY_ENABLED")) === "true";
const PROXY_ADMIN_URL = Cypress.env("PROXY_ADMIN_URL");
const PROXY_ADMIN_TIMEOUT_MS = 2000;
const REQUEST_ID_HEADER = "X-Request-ID";
const STEP_COUNTER_DIGITS = 3;

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

// Per-test trace state. Reset in beforeEach, then incremented once per
// cy.request so each outbound carries a deterministic X-Request-ID.
let testIdHash = "";
let stepCounter = 0;

// djb2 string hash rendered as 8-char hex. Deterministic across runs, so the
// same test always produces the same request-id namespace (and therefore
// matches the cassette it recorded earlier).
function stableHashHex(input) {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = ((hash * 33) ^ input.charCodeAt(i)) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

// A test's request-id namespace must be unique per (connector, spec, test):
//  - connector: parallel CI workers run the same test under different
//    connectors and must not share a namespace;
//  - spec: two spec files may reuse an identical describe/it titlePath;
//  - title: distinguishes tests within a spec.
// Without spec in the key, colliding rids merged into a bogus fan-out on replay.
function computeTestIdHash(connector, spec, title) {
  return stableHashHex(`${connector}:${spec}:${title}`);
}

// cy.request accepts several argument shapes (url; method+url; url+body;
// method+url+body; or a single options object). Collapse them all into one
// options object we can attach headers to.
function normalizeRequestArgs(args) {
  if (args.length === 1) {
    const [first] = args;
    if (typeof first === "string") return { url: first };
    if (first && typeof first === "object") return { ...first };
  }
  if (args.length === 2) {
    const [first, second] = args;
    if (typeof first === "string" && HTTP_METHODS.has(first.toUpperCase())) {
      return { method: first, url: second };
    }
    return { url: first, body: second };
  }
  if (args.length === 3) {
    return { method: args[0], url: args[1], body: args[2] };
  }
  return { url: args[0] };
}

// Proxy-admin calls (/test/start, /test/end, …) are control traffic, not
// recorded connector traffic, so they must not consume a step-counter slot.
function isProxyAdminUrl(url) {
  return Boolean(url) && url.startsWith(PROXY_ADMIN_URL);
}

// Advance the step counter and build the next deterministic request id.
function buildRequestId() {
  stepCounter += 1;
  const step = String(stepCounter).padStart(STEP_COUNTER_DIGITS, "0");
  return `${testIdHash}-${step}`;
}

function notifyProxyTestStarted(titlePath, spec, connector) {
  cy.request({
    method: "POST",
    url: `${PROXY_ADMIN_URL}/test/start`,
    body: {
      titlePath,
      spec,
      // Primary connector under test — used by capture to tag *every* flow
      // (including vault/auxiliary connector calls) under this connector
      // so they ship together in the same cassette tarball.
      connector,
      // Sent so the proxy can resolve `rid prefix → test context` even
      // for orphan late outbounds that arrive after /test/end (e.g.
      // async vault writes in External Vault save-card flows).
      testIdHash,
    },
    failOnStatusCode: false,
    timeout: PROXY_ADMIN_TIMEOUT_MS,
  });
}

function notifyProxyTestEnded() {
  cy.request({
    method: "POST",
    url: `${PROXY_ADMIN_URL}/test/end`,
    failOnStatusCode: false,
    timeout: PROXY_ADMIN_TIMEOUT_MS,
  });
}

// Only wire up the proxy hooks when IS_PROXY_ENABLED=true. Otherwise the
// suite runs against a live environment with no rid tagging or proxy chatter.
if (IS_PROXY_ENABLED) {
  beforeEach(() => {
    const { titlePath } = Cypress.currentTest;
    const title = titlePath.join(" > ");
    const connector = Cypress.env("CONNECTOR") || "";
    const spec = Cypress.spec.relative;

    testIdHash = computeTestIdHash(connector, spec, title);
    stepCounter = 0;

    if (PROXY_ADMIN_URL) {
      notifyProxyTestStarted(titlePath, spec, connector);
    }
  });

  afterEach(() => {
    if (PROXY_ADMIN_URL) {
      notifyProxyTestEnded();
    }
  });

  // Tag every cy.request with a deterministic X-Request-ID so the router can
  // propagate it onto connector outbounds and the proxy can match cassettes.
  Cypress.Commands.overwrite("request", (originalFn, ...args) => {
    const opts = normalizeRequestArgs(args);

    if (PROXY_ADMIN_URL && isProxyAdminUrl(opts.url)) {
      return originalFn(opts);
    }

    opts.headers = {
      ...(opts.headers || {}),
      [REQUEST_ID_HEADER]: buildRequestId(),
    };
    return originalFn(opts);
  });
}
