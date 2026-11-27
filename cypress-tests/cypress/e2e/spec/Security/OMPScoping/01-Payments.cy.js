import State from "../../../../utils/State";
import { OMP_ERRORS } from "../../../configs/Security/Commons";

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
let profileIdPB;
let jwtPA;
let paymentIdPA;
let paymentIdPB;
let paymentIdIncrementalPC;

describe("OMP Scoping - Payments", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
      paymentIdPA = globalState.get("paymentIdPA");
      paymentIdPB = globalState.get("paymentIdPB");
      paymentIdIncrementalPC = globalState.get("paymentIdIncrementalPC");
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

  context("Selector conflicts (R2)", () => {
    it("X-Profile-Id header cannot override a PA-scoped JWT's own profile", function () {
      // No dashboard JWT for M1 on this environment (magic-link-only
      // signin, see 00-Setup.cy.js) - skip rather than send "Bearer
      // undefined".
      if (!jwtPA) this.skip();
      // JWT scoped to PA, header claims PB - the JWT's own profile must
      // win; PB must not be reachable just by setting the header.
      cy.ompRetrievePayment(
        paymentIdPB,
        { type: "jwt", token: jwtPA, profileId: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("body.profile_id naming a DIFFERENT merchant's profile is rejected, not trusted at face value", () => {
      // M1's api key (no profile in header) - body.profile_id names
      // profileIdM2Default, a profile belonging to M2 entirely, not just
      // a same-merchant sibling profile (profileIdPB would be - that's
      // the existing PA-vs-PB coverage elsewhere in this file).
      cy.ompCreatePayment(
        { type: "apiKey", apiKey: apiKeyM1 },
        {
          amount: 1000,
          currency: "USD",
          profile_id: globalState.get("profileIdM2Default"),
        },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

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

    it("retrieve cross-profile PI_PB denied (JWT)", function () {
      if (!jwtPA) this.skip();
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayment(paymentIdPB),
        negativeRequest: () =>
          cy.ompRetrievePayment(
            paymentIdPB,
            { type: "jwt", token: jwtPA, profileId: profileIdPA },
            globalState
          ),
        // Confirmed live on current main: PR #15080 moved this check to a
        // profile-scoped DB query, so a cross-profile payment is a plain
        // not-found (HE_02), not the generic auth-layer IR_16 mismatch seen
        // on other resources - see the comment on
        // OMP_ERRORS.PAYMENT_PROFILE_SCOPED_NOT_FOUND for why this differs
        // from Business Profile's equivalent check.
        expectedError: OMP_ERRORS.PAYMENT_PROFILE_SCOPED_NOT_FOUND,
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

    // PI_PB can't be used for these two: both operations' own get_trackers
    // has an early payment-status gate that runs BEFORE the generic
    // cross-profile check (core_utils::validate_profile_id_from_auth_layer,
    // payments.rs:886) - incremental_authorization requires status
    // RequiresCapture plus incremental_authorization_allowed=true
    // (operations/payments_incremental_authorization.rs:85-96);
    // extend_authorization requires RequiresCapture or
    // PartiallyAuthorizedAndRequiresCapture
    // (operations/payments_extend_authorization.rs:78-85). PI_PB sits in
    // requires_payment_method (never confirmed), so a test against it would
    // short-circuit on that status gate instead of the real profile check -
    // a false pass, confirmed live (same pattern as the Connector Onboarding
    // and Dispute-fetch comments elsewhere in this folder).
    //
    // PI_INCREMENTAL_PC (00-Setup.cy.js) is a dedicated cybersource-backed
    // payment actually sitting in requires_capture with
    // incremental_authorization_allowed=true, so these two reach the real
    // check: PA's auth against a payment that belongs to profile PC.
    //
    // 2026-11-18: this setup step has been intermittently failing in this
    // environment with Cybersource returning "Authentication Failed" on
    // confirm - under investigation (possibly a credential-selection gate
    // tied to the running spec's filename, possibly a sandbox-side change;
    // NOT confirmed as a permanent limitation, since this reportedly
    // passed before). Keeping the real test active with a skip guard
    // rather than asserting a root cause prematurely.
    context("requires PI_INCREMENTAL_PC", () => {
      beforeEach(function () {
        if (!paymentIdIncrementalPC) this.skip();
      });

      it("incremental_authorization for cross-profile PI_INCREMENTAL_PC denied", () => {
        cy.ompAssertDeniedAndUnchanged({
          fetch: fetchPayment(paymentIdIncrementalPC),
          negativeRequest: () =>
            cy.ompIncrementalAuthorization(
              paymentIdIncrementalPC,
              { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
              { amount: 9000 },
              globalState
            ),
        });
      });

      it("extend_authorization for cross-profile PI_INCREMENTAL_PC denied", () => {
        cy.ompAssertDeniedAndUnchanged({
          fetch: fetchPayment(paymentIdIncrementalPC),
          negativeRequest: () =>
            cy.ompExtendAuthorization(
              paymentIdIncrementalPC,
              { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
              globalState
            ),
        });
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
