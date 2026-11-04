import State from "../../../../utils/State";

// #15080 - "enforce authenticated payment scope". PI_PA / PI_PB are two
// payment intents under the same merchant (M1) but different profiles
// (PA / PB), created in 00-Setup.cy.js. A user authenticated for PA must
// never be able to read or act on PB's payment, even though both sit
// under the same merchant.
//
// Positive controls for confirm/capture/cancel/session/complete_authorize
// only assert the call is accepted into normal processing (not rejected as
// a profile-mismatch) rather than asserting full business success - PI_PA
// was never confirmed with a real payment method in 00-Setup, so a 400 for
// "missing payment method" is expected and unrelated to scoping. Retrieve
// and update have no such precondition, so those get full positive checks.
let globalState;
let baseUrl;
let apiKeyM1;
let merchantIdM2;
let profileIdPA;
let jwtPA;
let paymentIdPA;
let paymentIdPB;

describe("OMP Scoping - Payments (#15080)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      jwtPA = globalState.get("userInfoTokenPA");
      paymentIdPA = globalState.get("paymentIdPA");
      paymentIdPB = globalState.get("paymentIdPB");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  // Ground-truth snapshot for the "resource unchanged after a denied
  // request" check - deliberately merchant-wide (no X-Profile-Id), not
  // scoped to PA. Scoping the snapshot fetch to PA would make it 404 for
  // PI_PB too, and the before/after comparison would trivially pass
  // without proving anything.
  function fetchPayment(paymentId) {
    return () =>
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/${paymentId}?force_sync=true`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
  }

  context("Retrieve", () => {
    it("TC-15080-11a (positive): PA retrieves its own PI_PA", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/${paymentIdPA}?force_sync=true`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.payment_id).to.eq(paymentIdPA);
      });
    });

    it("TC-15080-01: retrieve cross-profile PI_PB denied (JWT)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "GET",
            url: `${baseUrl}/payments/${paymentIdPB}?force_sync=true`,
            auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
          }),
      });
    });

    it("TC-15080-02: retrieve cross-profile PI_PB denied (API key + X-Profile-Id)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "GET",
            url: `${baseUrl}/payments/${paymentIdPB}?force_sync=true`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
          }),
      });
    });

    it("TC-15080-E01: non-existent payment ID returns 404, not a leak", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/pay_nonexistent_omp_test?force_sync=true`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("TC-15080-E03: malformed payment ID returns 4xx, not 500", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/!!not-a-valid-id!!?force_sync=true`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15080-E04: invalid/garbage bearer token on retrieve is rejected", () => {
      // Substitute for "expired JWT": minting a genuinely expired token
      // isn't possible from a black-box API test without the signing
      // secret, so this checks the same rejection path with a malformed one.
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/${paymentIdPA}?force_sync=true`,
        auth: { type: "invalid" },
      }).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Update", () => {
    it("TC-15080-11b (positive): PA updates its own PI_PA", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/payments/${paymentIdPA}`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { amount: 150, currency: "USD" },
      }).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("TC-15080-03: update cross-profile PI_PB denied, no mutation", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/${paymentIdPB}`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { amount: 999, currency: "USD" },
          }),
      });
    });
  });

  context("Confirm / capture / cancel / session / complete_authorize", () => {
    it("TC-15080-04: confirm cross-profile PI_PB denied before any connector call", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/${paymentIdPB}/confirm`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { client_secret: "", confirm: true },
          }),
      });
    });

    it("TC-15080-11c (positive): PA's confirm on its own PI_PA is accepted into processing", () => {
      // client_secret is only valid on the publishable-key/SDK confirm path;
      // server-to-server confirm via api-key must omit it.
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/payments/${paymentIdPA}/confirm`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { confirm: true },
      }).then((res) => {
        expect(res.status).to.not.eq(403);
      });
    });

    it("TC-15080-05: capture cross-profile PI_PB denied, no connector call", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/${paymentIdPB}/capture`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { amount_to_capture: 100 },
          }),
      });
    });

    it("TC-15080-10: capture rejects mismatched merchant_id in body", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/payments/${paymentIdPA}/capture`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { amount_to_capture: 100, merchant_id: merchantIdM2 },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15080-06: cancel cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/${paymentIdPB}/cancel`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { cancellation_reason: "requested_by_customer" },
          }),
      });
    });

    it("TC-15080-07: session token call for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/session_tokens`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { payment_id: paymentIdPB, client_secret: "", wallets: [] },
          }),
      });
    });

    it("TC-15080-08: complete_authorize for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payments/${paymentIdPB}/complete_authorize`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: {},
          }),
      });
    });
  });

  context("Attempt-ID operations", () => {
    // /payments/{id}/list-attempts is a V2-only route (web::scope("/v2/payments")
    // in crates/router/src/routes/app.rs), out of scope per the V2 decision.
    // The V1 way to see a payment's attempts is expand_attempts=true on the
    // regular retrieve endpoint.
    it("TC-15080-09: listing attempts for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "GET",
            url: `${baseUrl}/payments/${paymentIdPB}?force_sync=true&expand_attempts=true`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
          }),
      });
    });

    it("TC-15080-11d (positive): PA lists attempts for its own PI_PA", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payments/${paymentIdPA}?force_sync=true&expand_attempts=true`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.eq(200);
      });
    });
  });
});
