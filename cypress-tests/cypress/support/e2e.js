import "cypress-mochawesome-reporter/register";
import "./commands";
import "./redirectionHandler";

Cypress.on("window:before:load", (win) => {
  win.headers = {
    "Content-Security-Policy": "default-src 'self'",
    "X-Content-Type-Options": "nosniff",
    "X-Frame-Options": "DENY",
  };
});

Cypress.on("uncaught:exception", (err, runnable) => {
  // eslint-disable-next-line no-console
  console.error(
    `Error: ${err.message}\nError occurred in: ${runnable.title}\nStack trace: ${err.stack}`
  );
  return false;
});

// MITM proxy record/replay integration, gated by IS_PROXY_ENABLED. When on,
// each test notifies the proxy of start/end and tags every cy.request with a
// deterministic X-Request-ID that the router propagates to connector outbounds
// (the proxy's cassette match key on replay).
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

// Per-test trace state, reset in beforeEach.
let testIdHash = "";
let stepCounter = 0;

// djb2 hash as 8-char hex; deterministic so a test always reuses the same
// request-id namespace and matches its earlier recording.
function stableHashHex(input) {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = ((hash * 33) ^ input.charCodeAt(i)) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

// Namespace must be unique per (connector, spec, test) or rids collide on replay.
function computeTestIdHash(connector, spec, title) {
  return stableHashHex(`${connector}:${spec}:${title}`);
}

// Collapse cy.request's argument shapes into a single options object.
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

// Proxy-admin calls are control traffic and must not consume a step slot.
function isProxyAdminUrl(url) {
  return Boolean(url) && url.startsWith(PROXY_ADMIN_URL);
}

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
      connector,
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

  // Tag every cy.request with a deterministic X-Request-ID for cassette matching.
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
