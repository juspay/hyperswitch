import State from "../../../../utils/State";

// PI_PA / PI_PB are two payment intents under the same merchant (M1) but
// different profiles (PA / PB), created in 00-Setup.cy.js. A user
// authenticated for PA must never be able to read or act on PB's payment,
// even though both sit under the same merchant.
//
// Positive controls for confirm/capture/cancel/session/complete_authorize
// only assert the call is accepted into normal processing (not rejected as
// a profile-mismatch) rather than asserting full business success - PI_PA
// was never confirmed with a real payment method in 00-Setup, so a 400 for
// "missing payment method" is expected and unrelated to scoping. Retrieve
// and update have no such precondition, so those get full positive checks.
let globalState;
let apiKeyM1;
let merchantIdM2;
let profileIdPA;
let jwtPA;
let paymentIdPA;
let paymentIdPB;

describe("OMP Scoping - Payments", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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
      cy.ompRetrievePayment(
        paymentId,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
  }

  context("Retrieve", () => {
    it("PA retrieves its own PI_PA (positive)", () => {
      cy.ompRetrievePayment(
        paymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.payment_id).to.eq(paymentIdPA);
      });
    });

    it("retrieve cross-profile PI_PB denied (JWT)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRetrievePayment(
            paymentIdPB,
            { type: "jwt", token: jwtPA, profileId: profileIdPA },
            globalState
          ),
      });
    });

    it("retrieve cross-profile PI_PB denied (API key + X-Profile-Id)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRetrievePayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState
          ),
      });
    });

    it("non-existent payment ID returns 404, not a leak", () => {
      cy.ompRetrievePayment(
        "pay_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("malformed payment ID returns 4xx, not 500", () => {
      cy.ompRetrievePayment(
        "!!not-a-valid-id!!",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid/garbage bearer token on retrieve is rejected", () => {
      // Substitute for "expired JWT": minting a genuinely expired token
      // isn't possible from a black-box API test without the signing
      // secret, so this checks the same rejection path with a malformed one.
      cy.ompRetrievePayment(paymentIdPA, { type: "invalid" }, globalState).then(
        (res) => {
          expect(res.status).to.eq(401);
        }
      );
    });
  });

  context("Update", () => {
    it("PA updates its own PI_PA (positive)", () => {
      cy.ompUpdatePayment(
        paymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { amount: 150, currency: "USD" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("update cross-profile PI_PB denied, no mutation", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompUpdatePayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { amount: 999, currency: "USD" },
            globalState
          ),
      });
    });
  });

  context("Confirm / capture / cancel / session / complete_authorize", () => {
    it("confirm cross-profile PI_PB denied before any connector call", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompConfirmPayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { client_secret: "", confirm: true },
            globalState
          ),
      });
    });

    it("PA's confirm on its own PI_PA is accepted into processing (positive)", () => {
      // client_secret is only valid on the publishable-key/SDK confirm path;
      // server-to-server confirm via api-key must omit it. PI_PA has no
      // payment method attached (00-Setup.cy.js never confirmed it with real
      // card data), so this can't assert a single exact success status -
      // the business outcome legitimately varies (e.g. a missing-payment-
      // method 400). What this test actually has to prove is narrower: the
      // profile check passed, i.e. none of the denial statuses a
      // cross-profile request would get (401/403/404/412) showed up here.
      cy.ompConfirmPayment(
        paymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { confirm: true },
        globalState
      ).then((res) => {
        expect([401, 403, 404, 412]).to.not.include(res.status);
      });
    });

    it("capture cross-profile PI_PB denied, no connector call", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompCapturePayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { amount_to_capture: 100 },
            globalState
          ),
      });
    });

    it("capture rejects mismatched merchant_id in body", () => {
      cy.ompCapturePayment(
        paymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { amount_to_capture: 100, merchant_id: merchantIdM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("cancel cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompCancelPayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { cancellation_reason: "requested_by_customer" },
            globalState
          ),
      });
    });

    it("session token call for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompCreatePaymentSessionTokens(
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { payment_id: paymentIdPB, client_secret: "", wallets: [] },
            globalState
          ),
      });
    });

    it("complete_authorize for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompCompletePaymentAuthorize(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            {},
            globalState
          ),
      });
    });

    // No positive control for either of these two, same as capture/cancel/
    // session/complete_authorize above: PI_PA was never confirmed with a
    // real payment method, and incremental_authorization additionally needs
    // the payment created with request_incremental_authorization enabled
    // and sitting in requires_capture - neither precondition exists in this
    // suite's fixtures. The profile check runs before any of that state is
    // even inspected, so the denial case alone still proves the fix works.
    it("incremental_authorization for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompIncrementalAuthorization(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { amount: 200 },
            globalState
          ),
      });
    });

    it("extend_authorization for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompExtendAuthorization(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState
          ),
      });
    });
  });

  context("Attempt-ID operations", () => {
    // /payments/{id}/list-attempts is a V2-only route (web::scope("/v2/payments")
    // in crates/router/src/routes/app.rs), out of scope per the V2 decision.
    // The V1 way to see a payment's attempts is expand_attempts=true on the
    // regular retrieve endpoint.
    it("listing attempts for cross-profile PI_PB denied", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRetrievePayment(
            paymentIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState,
            { expandAttempts: true }
          ),
      });
    });

    it("PA lists attempts for its own PI_PA (positive)", () => {
      cy.ompRetrievePayment(
        paymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState,
        { expandAttempts: true }
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });
  });
});
