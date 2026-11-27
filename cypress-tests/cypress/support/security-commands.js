// Cypress custom commands used only by cypress/e2e/spec/Security/OMPScoping/.
// Split out of commands.js into its own file so the OMP (merchant/profile)
// scoping test helpers don't keep growing a 14k-line shared file - every
// command here is self-contained and doesn't depend on anything defined in
// commands.js. Loaded directly by cypress/support/e2e.js.
//
// auth variants:
//   { type: "apiKey", apiKey, profileId? }
//   { type: "adminApiKey", apiKey }       - AdminApiAuth/V2AdminApiAuth.
//                                            NOT the same header as
//                                            "apiKey" - confirmed live that
//                                            a plain api-key header on an
//                                            admin-only route returns 401,
//                                            since V2AdminApiAuth parses
//                                            the key from Authorization in
//                                            the exact form
//                                            "admin-api-key=<key>"
//                                            (services/authentication.rs:
//                                            1748-1755), not api-key.
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
    case "adminApiKey":
      headers["Authorization"] = `admin-api-key=${auth.apiKey}`;
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

  // lib.rs:171 headers::X_CONNECTED_MERCHANT_ID - lets a platform-type
  // merchant act "as" one of its own org's connected-type merchants.
  // Resolved server-side in resolve_platform()/validate_connected_merchant_account()
  // (services/authentication.rs) - same organization_id AND target
  // merchant_account_type == Connected, or it's rejected.
  if (auth.connectedMerchantId) {
    headers["x-connected-merchant-id"] = auth.connectedMerchantId;
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

Cypress.Commands.add("ompAcceptDispute", (disputeId, auth, globalState) =>
  ompCall("POST", `/disputes/accept/${disputeId}`, auth, {}, globalState)
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
  "ompDeleteBusinessProfile",
  (accountId, profileId, auth, globalState) =>
    ompCall(
      "DELETE",
      `/account/${accountId}/business_profile/${profileId}`,
      auth,
      undefined,
      globalState
    )
);

Cypress.Commands.add(
  "ompCreateBusinessProfileRaw",
  (accountId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/account/${accountId}/business_profile`,
      auth,
      body,
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

// --- OMP scoping: shared helpers for resources not yet exercised anywhere
// else in this suite (cypress/e2e/spec/Security/OMPScoping/) ---

// NOTE: there is deliberately no "create MCA with no profile_id" command
// here - confirmed live that connector creation always resolves a profile
// at creation time (explicitly via profile_id, or a legacy
// business_country/business_label fallback); omitting profile_id returns
// IR_04 "Missing required param: profile_id or business_country,
// business_label". A NULL profile_id isn't reachable through the public
// API, so the nullable-profile_id fail-closed check
// (validate_profile_id_from_auth_layer) is verified by reading its source
// only, not by an API-level test.

// --- OMP scoping: Customer (cypress/e2e/spec/Security/OMPScoping/11-CustomerPaymentMethodMandate.cy.js) ---

Cypress.Commands.add("ompRetrieveCustomer", (customerId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/customers/${customerId}`,
    auth,
  });
});

Cypress.Commands.add(
  "ompUpdateCustomer",
  (customerId, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/customers/${customerId}`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add("ompDeleteCustomer", (customerId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "DELETE",
    url: `${baseUrl}/customers/${customerId}`,
    auth,
  });
});

// --- OMP scoping: Payment Method ---

Cypress.Commands.add(
  "ompRetrievePaymentMethod",
  (paymentMethodId, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/payment_methods/${paymentMethodId}`,
      auth,
    });
  }
);

Cypress.Commands.add(
  "ompUpdatePaymentMethod",
  (paymentMethodId, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/payment_methods/${paymentMethodId}/update`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add(
  "ompDeletePaymentMethod",
  (paymentMethodId, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "DELETE",
      url: `${baseUrl}/payment_methods/${paymentMethodId}`,
      auth,
    });
  }
);

Cypress.Commands.add("ompCreatePaymentMethod", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/payment_methods`,
    auth,
    body,
  });
});

// --- OMP scoping: Mandate ---

Cypress.Commands.add("ompRetrieveMandate", (mandateId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/mandates/${mandateId}`,
    auth,
  });
});

Cypress.Commands.add("ompRevokeMandate", (mandateId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/mandates/revoke/${mandateId}`,
    auth,
  });
});

// --- OMP scoping: Refund (cypress/e2e/spec/Security/OMPScoping/12-Refund.cy.js) ---

Cypress.Commands.add("ompRetrieveRefund", (refundId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/refunds/${refundId}`,
    auth,
  });
});

Cypress.Commands.add("ompUpdateRefund", (refundId, auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/refunds/${refundId}`,
    auth,
    body,
  });
});

Cypress.Commands.add("ompCreateRefund", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/refunds`,
    auth,
    body,
  });
});

// --- OMP scoping: Routing retrieve/unlink (cypress/e2e/spec/Security/OMPScoping/14-Routing.cy.js) ---

Cypress.Commands.add(
  "ompRetrieveRoutingAlgorithm",
  (algorithmId, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/routing/${algorithmId}`,
      auth,
    });
  }
);

Cypress.Commands.add(
  "ompActivateRoutingAlgorithm",
  (algorithmId, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/routing/${algorithmId}/activate`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add("ompDeactivateRouting", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/routing/deactivate`,
    auth,
    body,
  });
});

// --- OMP scoping: Dynamic routing strategies (cypress/e2e/spec/Security/OMPScoping/25-DynamicRouting.cy.js) ---
// A separate route tree from the generic /routing endpoints above - gated
// behind the "dynamic_routing" cargo feature, path /account/{account_id}/
// business_profile/{profile_id}/dynamic_routing/* (app.rs:2722,2734), and
// accepting EITHER a plain api-key (HeaderAuth(ApiKeyAuth)) OR
// JWTAuthProfileFromRoute - not the JWTAuthMerchantAndProfileFromRoute used
// by the rest of Business Profile (routing.rs:1360-1366 et al).

Cypress.Commands.add(
  "ompCreateSuccessBasedRouting",
  (accountId, profileId, enable, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url:
        `${baseUrl}/account/${accountId}/business_profile/${profileId}` +
        `/dynamic_routing/success_based/create?enable=${enable}`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add(
  "ompCreateEliminationRouting",
  (accountId, profileId, enable, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url:
        `${baseUrl}/account/${accountId}/business_profile/${profileId}` +
        `/dynamic_routing/elimination/create?enable=${enable}`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add(
  "ompContractBasedRoutingToggle",
  (accountId, profileId, enable, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url:
        `${baseUrl}/account/${accountId}/business_profile/${profileId}` +
        `/dynamic_routing/contracts/toggle?enable=${enable}`,
      auth,
      body,
    });
  }
);

Cypress.Commands.add(
  "ompSetDynamicRoutingVolumeSplit",
  (accountId, profileId, split, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url:
        `${baseUrl}/account/${accountId}/business_profile/${profileId}` +
        `/dynamic_routing/set_volume_split?split=${split}`,
      auth,
    });
  }
);

Cypress.Commands.add(
  "ompGetDynamicRoutingVolumeSplit",
  (accountId, profileId, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "GET",
      url:
        `${baseUrl}/account/${accountId}/business_profile/${profileId}` +
        `/dynamic_routing/get_volume_split`,
      auth,
    });
  }
);

// --- OMP scoping: Platform/connected-merchant header (cypress/e2e/spec/Security/OMPScoping/26-PlatformConnectedMerchant.cy.js) ---
// X-Connected-Merchant-Id (lib.rs:171) - lets a platform-type merchant act
// as one of its own org's connected-type merchants. Resolved via
// resolve_platform()/validate_connected_merchant_account()
// (services/authentication.rs) - cross-org or non-Connected targets must be
// rejected. GET /payments/list is one of the openapi-documented routes that
// accepts this header and needs no pre-existing payment to exercise.
Cypress.Commands.add("ompListPayments", (auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/payments/list?limit=1`,
    auth,
  });
});

// --- OMP scoping: Analytics (cypress/e2e/spec/Security/OMPScoping/17-Analytics.cy.js) ---

// get_merchant_payment_metrics (crates/router/src/analytics.rs) deserializes
// web::Json<[GetPaymentMetricRequest; 1]> - a 1-element array, not a plain
// object - and GetPaymentMetricRequest itself is #[serde(rename_all =
// "camelCase")] (timeRange/groupByNames), while its nested TimeRange and
// PaymentFilters types stay snake_case. Callers here pass the plain request
// object; this command wraps it in the required array so every test
// doesn't need to know about that oddity.
Cypress.Commands.add("ompQueryPaymentMetrics", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/analytics/v1/metrics/payments`,
    auth,
    body: [body],
  });
});

// Org-level variant (get_org_payment_metrics, analytics.rs:628-669) - same
// array-wrapped GetPaymentMetricRequest body as the merchant-level route
// above, but AuthInfo is built from auth.platform.to_org_level_auth_info()
// rather than the request body, and the auth requirement is
// OrganizationAnalyticsRead (PlatformOrgAdminAuth or JWT), not
// MerchantAnalyticsRead.
Cypress.Commands.add("ompQueryOrgPaymentMetrics", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/analytics/v1/org/metrics/payments`,
    auth,
    body: [body],
  });
});

// --- OMP scoping: role-based user provisioning, for Analytics org_admin vs
// merchant_admin test cases (17-Analytics.cy.js). No existing Cypress
// command covers inviting a user or logging one in from scratch (the
// existing userLogin/terminate2Fa commands assume a single fixed
// globalState "email"/"password" pair from the initial dashboard signup,
// not a second, separately-provisioned user) - these are deliberately raw/
// low-level rather than folded into the existing 3-command signup chain,
// since the invited-user flow has an extra mandatory step
// (force_set_password) that signup doesn't.

Cypress.Commands.add("ompInviteUser", (auth, invitees, globalState) =>
  // Nested scope: outer web::scope("/user") (app.rs:2992) + inner
  // web::scope("/user") (app.rs:3180) = /user/user/invite_multiple,
  // confirmed against the route registration - not /user/invite_multiple.
  ompCall("POST", "/user/user/invite_multiple", auth, invitees, globalState)
);

Cypress.Commands.add("ompUserSignin", (email, password, globalState) =>
  ompCall(
    "POST",
    "/user/v2/signin?token_only=true",
    { type: "none" },
    { email, password },
    globalState
  )
);

Cypress.Commands.add("ompTerminate2FaRaw", (token, globalState) =>
  ompCall(
    "GET",
    "/user/2fa/terminate?skip_two_factor_auth=true",
    { type: "jwt", token },
    undefined,
    globalState
  )
);

Cypress.Commands.add("ompRotatePassword", (token, newPassword, globalState) =>
  ompCall(
    "POST",
    "/user/rotate_password",
    { type: "jwt", token },
    { password: newPassword },
    globalState
  )
);

// --- OMP scoping: Relay (cypress/e2e/spec/Security/OMPScoping/18-Relay.cy.js) ---

Cypress.Commands.add("ompRetrieveRelay", (relayId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/relay/${relayId}`,
    auth,
  });
});

Cypress.Commands.add("ompCreateRelay", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/relay`,
    auth,
    body,
  });
});

// --- OMP scoping: Payment Link / Payout Link retrieve
// (cypress/e2e/spec/Security/OMPScoping/19-PaymentLink.cy.js, 20-PayoutLink.cy.js) ---

Cypress.Commands.add(
  "ompRetrievePaymentLink",
  (paymentLinkId, auth, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/payment_link/${paymentLinkId}`,
      auth,
    });
  }
);

// render_payout_link uses auth::MerchantIdAuth(merchant_id) - the path's
// own merchant_id IS the authenticated identity, there is no separate
// bearer/api-key credential on this route at all (it's the public,
// customer-facing link page). So there's no "auth" parameter here - the
// cross-merchant check is entirely about whether a path merchant_id that
// doesn't own payoutId returns not-found.
Cypress.Commands.add(
  "ompRetrievePayoutLinkByPath",
  (merchantId, payoutId, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.request({
      method: "GET",
      url: `${baseUrl}/payout_link/${merchantId}/${payoutId}`,
      failOnStatusCode: false,
    });
  }
);

// --- OMP scoping: Surcharge Config raw get/put
// (cypress/e2e/spec/Security/OMPScoping/15-Surcharge.cy.js) - distinct from
// the existing createSurchargeDSLConfig/retrieveSurchargeDSLConfig
// commands, which take their own Response-comparison data shape; these are
// the plain ompRequest-style equivalents used for cross-merchant checks. ---

Cypress.Commands.add("ompGetSurchargeConfig", (auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/routing/decision/surcharge`,
    auth,
  });
});

Cypress.Commands.add("ompPutSurchargeConfig", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "PUT",
    url: `${baseUrl}/routing/decision/surcharge`,
    auth,
    body,
  });
});

// --- OMP scoping: generic payment create
// (cypress/e2e/spec/Security/OMPScoping/21-PaymentCrossReference.cy.js) ---

Cypress.Commands.add("ompCreatePayment", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/payments`,
    auth,
    body,
  });
});

// --- OMP scoping: generic payout/routing create
// (cypress/e2e/spec/Security/OMPScoping/27-ReferenceFields.cy.js) ---

Cypress.Commands.add("ompCreatePayout", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/payouts/create`,
    auth,
    body,
  });
});

Cypress.Commands.add("ompListPayouts", (auth, globalState) =>
  ompCall("GET", "/payouts/list", auth, undefined, globalState)
);

// Mirrors commands.js's addRoutingConfig body shape (routing-config-body.json
// + algorithm.type/data) but as a raw request, so it can be sent under an
// arbitrary auth identity without touching the shared globalState.apiKey the
// non-security addRoutingConfig command reads from.
Cypress.Commands.add(
  "ompCreateRoutingConfig",
  (auth, profileId, routingData, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/routing`,
      auth,
      body: {
        name: "omp scoping reference-field test",
        description: "created by 27-ReferenceFields.cy.js",
        profile_id: profileId,
        algorithm: { type: "priority", data: routingData },
      },
    });
  }
);

Cypress.Commands.add(
  "ompExtendPaymentAuthorization",
  (paymentId, auth, body, globalState) => {
    const baseUrl = globalState.get("baseUrl");
    return cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/payments/${paymentId}/extend_authorization`,
      auth,
      body,
    });
  }
);

// --- OMP scoping: MCA update/delete/list
// (cypress/e2e/spec/Security/OMPScoping/22-MCA.cy.js) - ompRetrieveMca
// already exists above, added for 06-ConnectorOnboarding.cy.js. ---

Cypress.Commands.add(
  "ompUpdateMca",
  (merchantId, mcaId, auth, body, globalState) =>
    ompCall(
      "POST",
      `/account/${merchantId}/connectors/${mcaId}`,
      auth,
      body,
      globalState
    )
);

Cypress.Commands.add("ompDeleteMca", (merchantId, mcaId, auth, globalState) =>
  ompCall(
    "DELETE",
    `/account/${merchantId}/connectors/${mcaId}`,
    auth,
    undefined,
    globalState
  )
);

Cypress.Commands.add("ompCreateMcaRaw", (merchantId, auth, body, globalState) =>
  ompCall("POST", `/account/${merchantId}/connectors`, auth, body, globalState)
);

Cypress.Commands.add("ompListMca", (merchantId, auth, globalState) =>
  ompCall(
    "GET",
    `/account/${merchantId}/connectors`,
    auth,
    undefined,
    globalState
  )
);

Cypress.Commands.add(
  "ompListProfileConnectors",
  (accountId, auth, globalState) =>
    ompCall(
      "GET",
      `/account/${accountId}/profile/connectors`,
      auth,
      undefined,
      globalState
    )
);

// --- OMP scoping: API Keys
// (cypress/e2e/spec/Security/OMPScoping/23-ApiKeys.cy.js) - distinct from
// ompCreateThrowawayApiKey/ompRevokeApiKey above, which are fixed to the
// admin-key auth scheme for test-fixture housekeeping; these take an
// arbitrary auth (JWT or admin key) for cross-merchant negative tests. ---

Cypress.Commands.add("ompCreateApiKey", (merchantId, auth, body, globalState) =>
  ompCall("POST", `/api_keys/${merchantId}`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompRetrieveApiKey",
  (merchantId, keyId, auth, globalState) =>
    ompCall(
      "GET",
      `/api_keys/${merchantId}/${keyId}`,
      auth,
      undefined,
      globalState
    )
);

Cypress.Commands.add(
  "ompUpdateApiKey",
  (merchantId, keyId, auth, body, globalState) =>
    ompCall("POST", `/api_keys/${merchantId}/${keyId}`, auth, body, globalState)
);

Cypress.Commands.add(
  "ompRevokeApiKeyAs",
  (merchantId, keyId, auth, globalState) =>
    ompCall(
      "DELETE",
      `/api_keys/${merchantId}/${keyId}`,
      auth,
      undefined,
      globalState
    )
);

Cypress.Commands.add("ompListApiKeys", (merchantId, auth, globalState) =>
  ompCall("GET", `/api_keys/${merchantId}/list`, auth, undefined, globalState)
);

// --- OMP scoping: Merchant Account
// (cypress/e2e/spec/Security/OMPScoping/24-MerchantAccount.cy.js) - distinct
// from ompGetMerchantAccount above, which is fixed to the admin-key auth
// scheme for test-fixture housekeeping (e.g. fetching publishable_key);
// these take an arbitrary auth for cross-merchant negative tests. ---

Cypress.Commands.add(
  "ompRetrieveMerchantAccount",
  (merchantId, auth, globalState) =>
    ompCall("GET", `/accounts/${merchantId}`, auth, undefined, globalState)
);

Cypress.Commands.add(
  "ompUpdateMerchantAccount",
  (merchantId, auth, body, globalState) =>
    ompCall("POST", `/accounts/${merchantId}`, auth, body, globalState)
);

// v1 POST /accounts is PlatformOrgAdminAuth{is_admin_auth_allowed:true} -
// admin.rs:242-245 - NOT a plain merchant API key route at all.
Cypress.Commands.add("ompCreateMerchantAccountRaw", (auth, body, globalState) =>
  ompCall("POST", "/accounts", auth, body, globalState)
);

// --- OMP scoping: Admin Theme (cypress/e2e/spec/Security/OMPScoping/29-Theme.cy.js) ---
// GET/POST /admin/theme - ThemeLineage is internally tagged on `entity_type`
// (common_utils/types/user/theme.rs:10-13) and deserialized via web::Query
// on GET, so it's sent as flat query params here.

Cypress.Commands.add("ompGetAdminTheme", (auth, lineageParams, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  const qs = new URLSearchParams(lineageParams).toString();
  return cy.ompRequest({
    method: "GET",
    // Nested under the OUTER /user scope (app.rs:2992,3294-3300) - the real
    // path is /user/admin/theme, not /admin/theme. Confirmed live
    // 2026-11-19: hitting /admin/theme directly gave a uniform 404 across
    // every credential type (including a valid admin key), which is
    // actix's own "no matching route" response, not an auth decision -
    // same doubly-nested-scope trap as /user/user/invite_multiple found
    // earlier this session.
    url: `${baseUrl}/user/admin/theme?${qs}`,
    auth,
  });
});

Cypress.Commands.add("ompGetThemeVersion", (themeId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    // Also nested under /user (app.rs:3310+) - real path is
    // /user/theme/{theme_id}/version. Handler uses auth::NoAuth
    // (theme.rs:343) - literally no auth guard at all.
    url: `${baseUrl}/user/theme/${themeId}/version`,
    auth,
  });
});

// POST /user/admin/theme - create_theme (theme.rs:81-96), AdminApiAuth
// only, body carries an arbitrary ThemeLineage directly (admin.rs's own
// priority finding). Used here to seed a REAL theme for the safe,
// user-scoped GET /theme/{theme_id} cross-org test, since nothing else
// in this suite creates one.
Cypress.Commands.add("ompCreateAdminTheme", (auth, body, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "POST",
    url: `${baseUrl}/user/admin/theme`,
    auth,
    body,
  });
});

// GET /user/theme/{theme_id} - get_user_theme_using_theme_id
// (theme.rs:166), JWTAuth{OrganizationThemeRead} + the real
// can_user_access_theme cross-match (utils/user/theme.rs:295).
Cypress.Commands.add("ompGetUserTheme", (themeId, auth, globalState) => {
  const baseUrl = globalState.get("baseUrl");
  return cy.ompRequest({
    method: "GET",
    url: `${baseUrl}/user/theme/${themeId}`,
    auth,
  });
});

// --- OMP scoping: Webhook delivery attempts (cypress/e2e/spec/Security/OMPScoping/30-WebhookEvents.cy.js) ---
// POST /events/{merchant_id} (list), GET/POST .../{event_id}/attempts,/retry
// - event_id ("initial_attempt_id") is a plain String, not a branded ID
// type (webhook_events.rs) - see Sheet 3/4 flag from the 2026-11-18 pass.

Cypress.Commands.add(
  "ompListWebhookDeliveryAttempts",
  (merchantId, auth, body, globalState) =>
    ompCall("POST", `/events/${merchantId}`, auth, body ?? {}, globalState)
);

Cypress.Commands.add(
  "ompGetWebhookDeliveryAttempts",
  (merchantId, eventId, auth, globalState) =>
    ompCall(
      "GET",
      `/events/${merchantId}/${eventId}/attempts`,
      auth,
      undefined,
      globalState
    )
);

Cypress.Commands.add(
  "ompRetryWebhookDeliveryAttempt",
  (merchantId, eventId, auth, globalState) =>
    ompCall(
      "POST",
      `/events/${merchantId}/${eventId}/retry`,
      auth,
      {},
      globalState
    )
);
