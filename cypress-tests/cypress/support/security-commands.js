// Cypress custom commands used only by cypress/e2e/spec/Security/OMPScoping/.
// Split out of commands.js into its own file so the OMP (merchant/profile)
// scoping test helpers don't keep growing a 14k-line shared file - every
// command here is self-contained and doesn't depend on anything defined in
// commands.js. Loaded directly by cypress/support/e2e.js.
//
// auth variants:
//   { type: "apiKey", apiKey, profileId? }
//   { type: "jwt", token, profileId? }
//   { type: "none" }                      - no Authorization/api-key header
//   { type: "invalid" }                   - garbage bearer token, substitutes
//                                            for an expired JWT (minting a
//                                            genuinely expired one isn't
//                                            possible from a black-box API
//                                            test without the signing secret)

function buildOmpHeaders(auth = {}) {
  const headers = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };

  switch (auth.type) {
    case "apiKey":
      headers["api-key"] = auth.apiKey;
      break;
    case "jwt":
      headers["Authorization"] = `Bearer ${auth.token}`;
      break;
    case "invalid":
      headers["Authorization"] = "Bearer not-a-real-token";
      break;
    case "none":
      break;
    default:
      throw new Error(`ompRequest: unknown auth.type "${auth.type}"`);
  }

  if (auth.profileId) {
    headers["X-Profile-Id"] = auth.profileId;
  }

  return headers;
}

Cypress.Commands.add(
  "ompRequest",
  ({ method, url, auth, body, failOnStatusCode = false }) => {
    return cy
      .request({
        method,
        url,
        headers: buildOmpHeaders(auth),
        body,
        failOnStatusCode,
      })
      .then((response) => {
        // Cypress.log, not cy.task: cy.task queues an async command, and
        // returning `response` synchronously right after it trips
        // Cypress's "mixing sync and async code" check.
        Cypress.log({
          name: "ompRequest",
          message: `${method} ${url} -> ${response.status}`,
        });
        return response;
      });
  }
);

// Housekeeping timestamps that get bumped as a side effect of merely
// reading a resource (e.g. force_sync=true re-touching a payment on every
// GET) - confirmed live on PI_INCREMENTAL_PC (a real confirmed/captured
// payment; PI_PA/PI_PB never left requires_payment_method, so this never
// surfaced there). Not meaningful mutation evidence for a security check -
// a denied request that merely caused a read-driven timestamp bump isn't a
// scoping bug.
const VOLATILE_RESOURCE_FIELDS = ["modified_at", "updated"];

// Asserts a cross-scope request was denied (anything but 2xx) and that the
// target resource is unchanged, per the test sheet's "evidence to retain"
// convention: fetch -> attempt the negative request -> re-fetch -> compare.
//
// Checks the full denial response, not just the status code:
// - Always: the body is a well-formed Hyperswitch error envelope
//   ({ error: { type, code, message } }, all non-empty strings) - catches a
//   "denial" that's actually an empty/malformed body rather than a real
//   error response, which `status >= 400` alone would miss.
// - If `expectedError` is given ({ status?, code?, message? }), each
//   provided field must match exactly - use this when the exact error is
//   known and worth pinning down (e.g. a specific IR_xx code), not just
//   "some 4xx happened."
Cypress.Commands.add(
  "ompAssertDeniedAndUnchanged",
  ({ fetch, negativeRequest, expectedError }) => {
    return fetch().then((before) => {
      return negativeRequest().then((negativeResponse) => {
        expect(
          negativeResponse.status,
          "negative request must be denied"
        ).to.be.gte(400);

        const errorBody = negativeResponse.body?.error;
        expect(
          errorBody,
          "denial response must carry a well-formed error envelope: " +
            JSON.stringify(negativeResponse.body)
        ).to.exist;
        expect(errorBody?.type, "error.type").to.be.a("string").and.not.be
          .empty;
        expect(errorBody?.code, "error.code").to.be.a("string").and.not.be
          .empty;
        expect(errorBody?.message, "error.message").to.be.a("string").and.not.be
          .empty;

        if (expectedError?.status !== undefined) {
          expect(negativeResponse.status, "expected exact status").to.eq(
            expectedError.status
          );
        }
        if (expectedError?.code !== undefined) {
          expect(errorBody?.code, "expected exact error.code").to.eq(
            expectedError.code
          );
        }
        if (expectedError?.message !== undefined) {
          expect(errorBody?.message, "expected exact error.message").to.eq(
            expectedError.message
          );
        }

        return fetch().then((after) => {
          // Chai's deep.equal failure dump gets truncated in the Cypress
          // reporter UI on anything but a small object, making it hard to
          // tell which field actually differs. Log that explicitly first so
          // a failure here is diagnosable without needing devtools.
          const changedKeys = Object.keys({ ...before.body, ...after.body })
            .filter((key) => !VOLATILE_RESOURCE_FIELDS.includes(key))
            .filter(
              (key) =>
                JSON.stringify(before.body?.[key]) !==
                JSON.stringify(after.body?.[key])
            )
            .map((key) => ({
              key,
              before: before.body?.[key],
              after: after.body?.[key],
            }));

          if (changedKeys.length > 0) {
            Cypress.log({
              name: "ompAssertDeniedAndUnchanged",
              message: `fields changed despite denial: ${JSON.stringify(changedKeys)}`,
            });
          }

          expect(
            changedKeys,
            "target resource must be unchanged after a denied request - " +
              "changed fields: " +
              JSON.stringify(changedKeys)
          ).to.deep.equal([]);
        });
      });
    });
  }
);

// Creates a throwaway merchant API key using the admin API key (key
// management is always admin-authenticated, never via a merchant's own
// api-key - see getManagementAuthHeaders above). Used by the "revoked API
// key is rejected" edge cases in cypress/e2e/spec/Security/OMPScoping/.
Cypress.Commands.add("ompCreateThrowawayApiKey", (merchantId, globalState) => {
  const adminApiKey = globalState.get("adminApiKey");
  const baseUrl = globalState.get("baseUrl");

  return cy
    .request({
      method: "POST",
      url: `${baseUrl}/api_keys/${merchantId}`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        Authorization: `admin-api-key=${adminApiKey}`,
      },
      body: {
        name: "omp-scoping-throwaway-key",
        expiration: "2069-09-23T01:02:03.000Z",
      },
      failOnStatusCode: false,
    })
    .then((response) => {
      expect(response.status, "throwaway API key creation must succeed").to.eq(
        200
      );
      return { apiKey: response.body.api_key, keyId: response.body.key_id };
    });
});

// Revokes a merchant API key via the admin API key - pair for
// ompCreateThrowawayApiKey above.
Cypress.Commands.add("ompRevokeApiKey", (merchantId, keyId, globalState) => {
  const adminApiKey = globalState.get("adminApiKey");
  const baseUrl = globalState.get("baseUrl");

  return cy
    .request({
      method: "DELETE",
      url: `${baseUrl}/api_keys/${merchantId}/${keyId}`,
      headers: { Authorization: `admin-api-key=${adminApiKey}` },
      failOnStatusCode: false,
    })
    .then((response) => {
      expect(response.status, "API key revocation must succeed").to.eq(200);
      return response;
    });
});

// Fetches a merchant account via the admin API key, for reading fields
// (e.g. publishable_key) that aren't captured anywhere else when the
// merchant was created via the dashboard signup flow rather than
// merchantCreateCallTest. Used by cypress/e2e/spec/Security/OMPScoping/.
Cypress.Commands.add("ompGetMerchantAccount", (merchantId, globalState) => {
  const adminApiKey = globalState.get("adminApiKey");
  const baseUrl = globalState.get("baseUrl");

  return cy.request({
    method: "GET",
    url: `${baseUrl}/accounts/${merchantId}`,
    // V2AdminApiAuth parses the admin key from Authorization in the exact
    // form "admin-api-key=<key>" (services/authentication.rs:1748-1755) - a
    // plain api-key header returns 401, confirmed live. Same scheme as
    // ompCreateThrowawayApiKey/ompRevokeApiKey above.
    headers: { Authorization: `admin-api-key=${adminApiKey}` },
    failOnStatusCode: false,
  });
});

// Shared by every OMP endpoint command below - consolidates the
// repeated "read baseUrl, build the full URL, call cy.ompRequest" pattern
// into one place instead of duplicating it per command, per review
// feedback. Each command stays a separately named, purpose-built function
// (per the earlier "define cy.request in commands.js" feedback); only the
// boilerplate inside each one is shared now.
function ompCall(method, path, auth, body, globalState) {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({ method, url: `${baseUrl}${path}`, auth, body });
}

// --- OMP scoping: Payments (cypress/e2e/spec/Security/OMPScoping/01-Payments.cy.js) ---

Cypress.Commands.add(
  "ompRetrievePayment",
  (paymentId, auth, globalState, options = {}) => {
    const expand = options.expandAttempts ? "&expand_attempts=true" : "";
    return ompCall(
      "GET",
      `/payments/${paymentId}?force_sync=true${expand}`,
      auth,
      undefined,
      globalState
    );
  }
);

Cypress.Commands.add("ompUpdatePayment", (paymentId, auth, body, globalState) =>
  ompCall("POST", `/payments/${paymentId}`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompConfirmPayment",
  (paymentId, auth, body, globalState) =>
    ompCall("POST", `/payments/${paymentId}/confirm`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompCapturePayment",
  (paymentId, auth, body, globalState) =>
    ompCall("POST", `/payments/${paymentId}/capture`, auth, body, globalState)
);

Cypress.Commands.add("ompCancelPayment", (paymentId, auth, body, globalState) =>
  ompCall("POST", `/payments/${paymentId}/cancel`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompIncrementalAuthorization",
  (paymentId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/payments/${paymentId}/incremental_authorization`,
      auth,
      body,
      globalState
    )
);

Cypress.Commands.add("ompExtendAuthorization", (paymentId, auth, globalState) =>
  ompCall(
    "POST",
    `/payments/${paymentId}/extend_authorization`,
    auth,
    {},
    globalState
  )
);

Cypress.Commands.add(
  "ompCreatePaymentSessionTokens",
  (auth, body, globalState) =>
    ompCall("POST", "/payments/session_tokens", auth, body, globalState)
);

Cypress.Commands.add(
  "ompCompletePaymentAuthorize",
  (paymentId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/payments/${paymentId}/complete_authorize`,
      auth,
      body,
      globalState
    )
);

// --- OMP scoping: Payouts (cypress/e2e/spec/Security/OMPScoping/02-Payouts.cy.js) ---

Cypress.Commands.add("ompRetrievePayout", (payoutId, auth, globalState) =>
  ompCall("GET", `/payouts/${payoutId}`, auth, undefined, globalState)
);

Cypress.Commands.add("ompUpdatePayout", (payoutId, auth, body, globalState) =>
  ompCall("PUT", `/payouts/${payoutId}`, auth, body, globalState)
);

Cypress.Commands.add("ompConfirmPayout", (payoutId, auth, body, globalState) =>
  ompCall("POST", `/payouts/${payoutId}/confirm`, auth, body, globalState)
);

Cypress.Commands.add("ompCancelPayout", (payoutId, auth, body, globalState) =>
  ompCall("POST", `/payouts/${payoutId}/cancel`, auth, body, globalState)
);

Cypress.Commands.add("ompFulfillPayout", (payoutId, auth, body, globalState) =>
  ompCall("POST", `/payouts/${payoutId}/fulfill`, auth, body, globalState)
);

// --- OMP scoping: Profile Acquirer (cypress/e2e/spec/Security/OMPScoping/03-ProfileAcquirer.cy.js) ---

Cypress.Commands.add("ompCreateProfileAcquirer", (auth, body, globalState) =>
  ompCall("POST", "/profile_acquirer", auth, body, globalState)
);

Cypress.Commands.add(
  "ompUpdateProfileAcquirer",
  (profileId, acquirerId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/profile_acquirer/${profileId}/${acquirerId}`,
      auth,
      body,
      globalState
    )
);

// --- OMP scoping: Authentication (cypress/e2e/spec/Security/OMPScoping/04-Authentication.cy.js) ---

Cypress.Commands.add("ompCreateAuthentication", (auth, body, globalState) =>
  ompCall("POST", "/authentication", auth, body, globalState)
);

Cypress.Commands.add(
  "ompAuthenticationEligibility",
  (authenticationId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/authentication/${authenticationId}/eligibility`,
      auth,
      body,
      globalState
    )
);

Cypress.Commands.add(
  "ompAuthenticationSync",
  (merchantId, authenticationId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/authentication/${merchantId}/${authenticationId}/sync`,
      auth,
      body,
      globalState
    )
);

// --- OMP scoping: Disputes (cypress/e2e/spec/Security/OMPScoping/05-Disputes.cy.js) ---

Cypress.Commands.add("ompListDisputes", (auth, globalState) =>
  ompCall("GET", "/disputes/list", auth, undefined, globalState)
);

Cypress.Commands.add("ompRetrieveDispute", (disputeId, auth, globalState) =>
  ompCall("GET", `/disputes/${disputeId}`, auth, undefined, globalState)
);

Cypress.Commands.add(
  "ompDeleteDisputeEvidence",
  (disputeId, auth, globalState) =>
    ompCall(
      "DELETE",
      "/disputes/evidence",
      auth,
      { dispute_id: disputeId },
      globalState
    )
);

Cypress.Commands.add(
  "ompFetchDisputesFromConnector",
  (connectorId, auth, globalState, fetchWindow = {}) => {
    // fetch_from/fetch_till are REQUIRED query params
    // (crates/router/src/types/api/disputes.rs:16-18, format
    // YYYY-MM-DDTHH:MM:SS) and are deserialized before auth even runs -
    // omitting them returns 400 "missing field `fetch_from`" regardless of
    // credentials or profile, which would make every test using this
    // command pass for the wrong reason (confirmed live). Default window is
    // the last 24 hours so every call reaches the real profile check.
    const toQueryFormat = (date) => date.toISOString().slice(0, 19);
    const now = new Date();
    const fetchTill = fetchWindow.fetchTill || toQueryFormat(now);
    const fetchFrom =
      fetchWindow.fetchFrom ||
      toQueryFormat(new Date(now.getTime() - 24 * 60 * 60 * 1000));

    return ompCall(
      "GET",
      `/disputes/${connectorId}/fetch?fetch_from=${fetchFrom}&fetch_till=${fetchTill}`,
      auth,
      undefined,
      globalState
    );
  }
);

// --- OMP scoping: Connector Onboarding + MCA retrieve
// (cypress/e2e/spec/Security/OMPScoping/06-ConnectorOnboarding.cy.js) ---

Cypress.Commands.add("ompRetrieveMca", (merchantId, mcaId, auth, globalState) =>
  ompCall(
    "GET",
    `/account/${merchantId}/connectors/${mcaId}`,
    auth,
    undefined,
    globalState
  )
);

Cypress.Commands.add("ompConnectorOnboardingSync", (auth, body, globalState) =>
  ompCall("POST", "/connector_onboarding/sync", auth, body, globalState)
);

Cypress.Commands.add(
  "ompConnectorOnboardingActionUrl",
  (auth, body, globalState) =>
    ompCall("POST", "/connector_onboarding/action_url", auth, body, globalState)
);

Cypress.Commands.add(
  "ompConnectorOnboardingResetTrackingId",
  (auth, body, globalState) =>
    ompCall(
      "POST",
      "/connector_onboarding/reset_tracking_id",
      auth,
      body,
      globalState
    )
);

// --- OMP scoping: Apple Pay (cypress/e2e/spec/Security/OMPScoping/07-ApplePay.cy.js) ---

Cypress.Commands.add(
  "ompVerifyApplePay",
  (merchantId, auth, body, globalState) =>
    ompCall("POST", `/verify/apple_pay/${merchantId}`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompGetApplePayVerifiedDomains",
  (merchantId, mcaId, auth, globalState) =>
    ompCall(
      "GET",
      `/verify/applepay_verified_domains?merchant_id=${merchantId}&merchant_connector_account_id=${mcaId}`,
      auth,
      undefined,
      globalState
    )
);

// --- OMP scoping: Business Profile (cypress/e2e/spec/Security/OMPScoping/08-BusinessProfile.cy.js) ---

Cypress.Commands.add(
  "ompRetrieveBusinessProfile",
  (accountId, profileId, auth, globalState) =>
    ompCall(
      "GET",
      `/account/${accountId}/business_profile/${profileId}`,
      auth,
      undefined,
      globalState
    )
);

Cypress.Commands.add(
  "ompUpdateBusinessProfile",
  (accountId, profileId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/account/${accountId}/business_profile/${profileId}`,
      auth,
      body,
      globalState
    )
);

Cypress.Commands.add(
  "ompToggleConnectorAgnosticMit",
  (accountId, profileId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/account/${accountId}/business_profile/${profileId}/toggle_connector_agnostic_mit`,
      auth,
      body,
      globalState
    )
);

// --- OMP scoping: Blocklist (cypress/e2e/spec/Security/OMPScoping/09-Blocklist.cy.js) ---

Cypress.Commands.add("ompGetBlocklist", (auth, queryString, globalState) =>
  ompCall("GET", `/blocklist${queryString}`, auth, undefined, globalState)
);

Cypress.Commands.add("ompCreateBlocklistEntry", (auth, body, globalState) =>
  ompCall("POST", "/blocklist", auth, body, globalState)
);

Cypress.Commands.add("ompDeleteBlocklistEntry", (auth, body, globalState) =>
  ompCall("DELETE", "/blocklist", auth, body, globalState)
);

Cypress.Commands.add("ompLookupBlocklist", (auth, data, globalState) =>
  ompCall(
    "GET",
    `/blocklist/lookup?data=${encodeURIComponent(data)}`,
    auth,
    undefined,
    globalState
  )
);
