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
    path: "redirect/response/stripe",
    params: (gs) => ({
      payment_intent: gs.get("connectorTransactionID"),
      redirect_status: "succeeded",
    }),
  },
  stripeconnect: {
    path: "redirect/response/stripe",
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

function callbackSegment(url) {
  return new URL(url).pathname
    .replace(/\/redirect\/token\/[^/]+\//, "/redirect/")
    .split("/redirect/")[1]
    ?.split("/callback/")[0];
}

function resolveCallbackUrl(globalState, saved, pageCallbacks = []) {
  const segment = saved?.__redirect_segment
    ?.replace(/^redirect\/(?:token\/[^/]+\/)?/, "")
    .split("/callback/")[0];
  const paymentId = globalState.get("paymentID");
  const merchantId = globalState.get("merchantId");
  const selectUrl = (urls) => {
    const matches = [...new Set(urls)].filter((url) => {
      const path = new URL(url).pathname;
      return (
        path.startsWith(
          `/payments/${paymentId}/${merchantId}/redirect/token/`
        ) &&
        (!segment || callbackSegment(url) === segment)
      );
    });
    expect(
      matches,
      "one fresh router-issued callback for this payment"
    ).to.have.length(1);
    return matches[0];
  };

  const matchingPageCallbacks = pageCallbacks.filter(
    (url) => !segment || callbackSegment(url) === segment
  );
  if (matchingPageCallbacks.length) {
    return cy.wrap(selectUrl(matchingPageCallbacks), { log: false });
  }

  const adminUrl = Cypress.env("PROXY_ADMIN_URL");
  expect(adminUrl, "replay proxy admin URL").to.be.a("string").and.not.be.empty;
  return cy
    .request({
      url: `${adminUrl}/callbacks`,
      qs: { payment_id: paymentId, merchant_id: merchantId },
      log: false,
    })
    .then((response) => selectUrl(response.body.urls));
}

function replayRedirect(
  connectorId,
  globalState,
  redirectBodyFile,
  pageCallbacks
) {
  const jsConnector = JS_IFRAME_CONNECTORS[connectorId];
  return cy.task("readFileOrNull", redirectBodyFile).then((saved) => {
    if (!saved && !jsConnector) {
      if (Cypress._buildRequestId) Cypress._buildRequestId();
      return;
    }
    const params = jsConnector?.params(globalState);
    if (!saved && params && Object.values(params).some((v) => v == null)) {
      if (Cypress._buildRequestId) Cypress._buildRequestId();
      return;
    }

    // Cassette metadata identifies the route and connector payload only. The
    // token must come from this run's redirect page or outbound connector call.
    const route = saved?.__redirect_segment
      ? saved
      : {
          __redirect_segment:
            jsConnector?.path || `redirect/complete/${connectorId}`,
        };
    return resolveCallbackUrl(globalState, route, pageCallbacks).then((url) => {
      const method = saved?.__redirect_method || (jsConnector ? "GET" : "POST");
      const callback = new URL(url);
      if (method === "GET") {
        const query = { ...(saved?.__query || {}), ...params };
        Object.entries(query).forEach(([key, value]) => {
          callback.searchParams.set(key, value);
        });
      }
      return cy
        .request({
          method,
          url: callback.toString(),
          ...(method === "POST" && {
            form: true,
            body: saved?.__body || saved,
          }),
          followRedirect: false,
          failOnStatusCode: false,
          retryOnNetworkFailure: false,
          log: false,
        })
        .then((response) => {
          expect(
            response.status,
            "fresh callback must authenticate"
          ).to.be.within(200, 399);
          if (response.headers.location) {
            globalState.set("redirectReturnUrl", response.headers.location);
          }
        });
    });
  });
}

export function mockReplay3ds(globalState, connectorId, nextActionUrl) {
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const seq = nextRedirectSeq(testIdHash);
  const redirectBodyFile = getRedirectBodyFile(connectorId, testIdHash, seq);

  return cy
    .request({
      url: nextActionUrl,
      failOnStatusCode: false,
      followRedirect: false,
      retryOnNetworkFailure: false,
      log: false,
    })
    .then((page) => {
      const document = new DOMParser().parseFromString(page.body, "text/html");
      const callbacks = [
        document.body.getAttribute("data-callback-url"),
        document.body.getAttribute("data-complete-callback-url"),
      ].filter(Boolean);
      return replayRedirect(
        connectorId,
        globalState,
        redirectBodyFile,
        callbacks
      );
    });
}

export function mockReplayBankRedirect(globalState, connectorId) {
  const testIdHash = Cypress.env("currentTestIdHash") || "unknown";
  const seq = nextRedirectSeq(testIdHash);
  const redirectBodyFile = getRedirectBodyFile(connectorId, testIdHash, seq);

  return replayRedirect(connectorId, globalState, redirectBodyFile);
}
