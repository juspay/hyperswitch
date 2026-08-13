/**
 * MITM proxy helpers for record/replay of 3DS and bank-redirect flows.
 *
 * Record mode  – REDIRECT_PROXY_ADMIN_URL is set, MOCK_SERVER is false.
 *                Cypress reserves a RID with the redirect proxy before the
 *                browser navigates to the ACS, so the proxy can inject it on
 *                the return leg and tie the outbound connector call to the
 *                right cassette.
 *
 * Replay mode  – IS_PROXY_ENABLED=true, MOCK_SERVER=true.
 *                The browser flow is skipped; Cypress reads the saved redirect
 *                body from CAPTURE_DIR and POST/GETs it directly to the router.
 */

// ── Connectors whose redirect return is a GET to a known path ────────────────

const JS_IFRAME_CONNECTORS = {
  stripe: {
    path: "redirect/response",
    params: (gs) => ({
      payment_intent: gs.get("connectorTransactionID"),
      redirect_status: "succeeded",
    }),
  },
  stripeconnect: {
    path: "redirect/response",
    params: (gs) => ({
      payment_intent: gs.get("connectorTransactionID"),
      redirect_status: "succeeded",
    }),
  },
};

// ── Module-level sequence counter (reset per test via resetMitmRedirectSeq) ──

const _redirectReadCount = {};

// ── Mode checks ──────────────────────────────────────────────────────────────

export function isMockServer() {
  return String(Cypress.env("MOCK_SERVER")) === "true";
}

export function isRecordMode() {
  return !!Cypress.env("REDIRECT_PROXY_ADMIN_URL") && !isMockServer();
}

export function isReplayMode() {
  return String(Cypress.env("IS_PROXY_ENABLED")) === "true" && isMockServer();
}

// Returns false only when explicitly set to false (no cassette found for this test).
// Defaults to true so non-proxy tests are unaffected.
export function hasCassetteForCurrentTest() {
  return Cypress.env("currentTestHasCassette") !== false;
}

// ── Sequence counter ─────────────────────────────────────────────────────────

export function resetMitmRedirectSeq(testIdHash) {
  delete _redirectReadCount[testIdHash];
}

function nextRedirectSeq(testIdHash) {
  _redirectReadCount[testIdHash] = (_redirectReadCount[testIdHash] || 0) + 1;
  return String(_redirectReadCount[testIdHash]).padStart(3, "0");
}

function getRedirectBodyFile(connectorId, testIdHash, seq) {
  const captureDir = Cypress.env("CAPTURE_DIR");
  return `${captureDir}/${connectorId}/Payment/redirect-bodies/${testIdHash}-${seq}-redirect-body.json`;
}

// ── Record helpers ───────────────────────────────────────────────────────────

function reserveRedirectRid(testIdHash, paymentId, stepOffset) {
  const adminUrl = Cypress.env("REDIRECT_PROXY_ADMIN_URL");
  const currentStep = Cypress._getStepCounter ? Cypress._getStepCounter() : 0;
  const rid = `${testIdHash}-${String(currentStep + stepOffset).padStart(3, "0")}`;
  cy.request({
    method: "POST",
    url: `${adminUrl}/reserve`,
    body: { rid, testIdHash, paymentId },
    failOnStatusCode: false,
    timeout: 2000,
  });
}

export function mockRecord3ds(
  globalState,
  nextActionUrl,
  expectedRedirection,
  handleRedirectionFn
) {
  const connectorId = globalState.get("connectorId");
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const paymentId = globalState.get("paymentID");

  reserveRedirectRid(testIdHash, paymentId, 2);

  handleRedirectionFn(
    "three_ds",
    {
      redirectionUrl: new URL(nextActionUrl),
      expectedUrl: new URL(expectedRedirection),
    },
    connectorId,
    globalState.get("paymentMethodType")
  );
  cy.then(() => {
    if (Cypress._buildRequestId) Cypress._buildRequestId();
  });
}

export function mockRecordBankRedirect(
  globalState,
  nextActionUrl,
  expectedRedirection,
  paymentMethodType,
  handleRedirectionFn
) {
  const connectorId = globalState.get("connectorId");
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const paymentId = globalState.get("paymentID");

  reserveRedirectRid(testIdHash, paymentId, 1);

  handleRedirectionFn(
    "bank_redirect",
    {
      redirectionUrl: new URL(nextActionUrl),
      expectedUrl: new URL(expectedRedirection),
    },
    connectorId,
    paymentMethodType
  );
  cy.then(() => {
    if (Cypress._buildRequestId) Cypress._buildRequestId();
  });
}

// ── Replay helpers ───────────────────────────────────────────────────────────

function replayRedirectBody(saved, paymentId, merchantId, notificationUrl) {
  const hyperswitchUrl =
    Cypress.env("HYPERSWITCH_URL") || "http://localhost:8080";
  if (saved && saved.__redirect_method === "GET") {
    const qs = new URLSearchParams(saved.__query || {}).toString();
    const base = saved.__redirect_segment
      ? `${hyperswitchUrl}/payments/${paymentId}/${merchantId}/${saved.__redirect_segment}`
      : notificationUrl;
    cy.request({
      method: "GET",
      url: qs ? `${base}?${qs}` : base,
      failOnStatusCode: false,
      followRedirect: false,
    });
  } else if (saved) {
    const postBody =
      saved.__redirect_method === "POST" && saved.__body ? saved.__body : saved;
    const postUrl = saved.__redirect_segment
      ? `${hyperswitchUrl}/payments/${paymentId}/${merchantId}/${saved.__redirect_segment}`
      : notificationUrl;
    cy.request({
      method: "POST",
      url: postUrl,
      form: true,
      body: postBody,
      failOnStatusCode: false,
      followRedirect: false,
    });
  } else {
    cy.then(() => {
      if (Cypress._buildRequestId) Cypress._buildRequestId();
    });
  }
}

function replayJsConnectorRedirect(
  jsConnector,
  saved,
  paymentId,
  merchantId,
  globalState
) {
  const hyperswitchUrl =
    Cypress.env("HYPERSWITCH_URL") || "http://localhost:8080";
  if (saved && saved.__redirect_method === "GET" && saved.__redirect_segment) {
    const url = `${hyperswitchUrl}/payments/${paymentId}/${merchantId}/${saved.__redirect_segment}`;
    const qs = new URLSearchParams(saved.__query || {}).toString();
    cy.request({
      method: "GET",
      url: qs ? `${url}?${qs}` : url,
      failOnStatusCode: false,
      followRedirect: false,
    });
  } else {
    const returnUrl = `${hyperswitchUrl}/payments/${paymentId}/${merchantId}/${jsConnector.path}/${globalState.get("connectorId")}`;
    const params = jsConnector.params(globalState);
    const hasValues = Object.values(params).every(
      (v) => v !== undefined && v !== null
    );
    if (hasValues) {
      const qs = new URLSearchParams(params).toString();
      cy.request({
        method: "GET",
        url: `${returnUrl}?${qs}`,
        failOnStatusCode: false,
        followRedirect: false,
      });
    } else {
      cy.then(() => {
        if (Cypress._buildRequestId) Cypress._buildRequestId();
      });
    }
  }
}

function replayRedirect(
  connectorId,
  globalState,
  redirectBodyFile,
  notificationUrl
) {
  const paymentId = globalState.get("paymentID");
  const merchantId = globalState.get("merchantId");
  const jsConnector = JS_IFRAME_CONNECTORS[connectorId];

  if (jsConnector) {
    cy.task("readFileOrNull", redirectBodyFile).then((saved) => {
      replayJsConnectorRedirect(
        jsConnector,
        saved,
        paymentId,
        merchantId,
        globalState
      );
    });
    return;
  }

  cy.task("readFileOrNull", redirectBodyFile).then((saved) => {
    replayRedirectBody(saved, paymentId, merchantId, notificationUrl);
  });
}

export function mockReplay3ds(globalState, connectorId, nextActionUrl) {
  const baseUrl = globalState.get("baseUrl");
  const paymentId = globalState.get("paymentID");
  const merchantId = globalState.get("merchantId");
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const seq = nextRedirectSeq(testIdHash);
  const redirectBodyFile = getRedirectBodyFile(connectorId, testIdHash, seq);
  const notificationUrl = `${baseUrl}/payments/${paymentId}/${merchantId}/redirect/complete/${connectorId}`;

  cy.request({
    url: nextActionUrl,
    failOnStatusCode: false,
    followRedirect: false,
  });
  replayRedirect(connectorId, globalState, redirectBodyFile, notificationUrl);
}

export function mockReplayBankRedirect(globalState, connectorId) {
  const baseUrl = globalState.get("baseUrl");
  const paymentId = globalState.get("paymentID");
  const merchantId = globalState.get("merchantId");
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const seq = nextRedirectSeq(testIdHash);
  const redirectBodyFile = getRedirectBodyFile(connectorId, testIdHash, seq);
  const notificationUrl = `${baseUrl}/payments/${paymentId}/${merchantId}/redirect/complete/${connectorId}`;

  replayRedirect(connectorId, globalState, redirectBodyFile, notificationUrl);
}
