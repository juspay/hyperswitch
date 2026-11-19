import State from "../../../../utils/State";

// PO_PA / PO_PB are payouts under M1, one per profile, created in
// 00-Setup.cy.js (skipped there if the configured payout connector has no
// credentials - gate on the same payoutsExecution flag here, matching the
// existing Payout/*.cy.js convention in this repo).
//
// confirm/cancel/fulfill are ApiKeyAuth-only routes (verified against
// crates/router/src/routes/app.rs) - there is no JWT alternative, so these
// negative tests use the merchant API key plus X-Profile-Id.
let globalState;
let apiKeyM1;
let merchantIdM2;
let profileIdPA;
let profileIdPB;
let payoutIdPA;
let payoutIdPB;
let payoutsExecution;

describe("OMP Scoping - Payouts", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      payoutIdPA = globalState.get("payoutIdPA");
      payoutIdPB = globalState.get("payoutIdPB");
      payoutsExecution = globalState.get("payoutsExecution");

      if (!payoutsExecution) {
        cy.task(
          "cli_log",
          "Skipping this suite: no payout_processor credentials in the " +
            "creds file (see 00-Setup.cy.js)."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!payoutsExecution) this.skip();
  });

  // Ground-truth snapshot, deliberately merchant-wide (no X-Profile-Id) so
  // comparing PB's "before"/"after" state isn't trivially masked by PA's
  // own denied access to it.
  function fetchPayout(payoutId) {
    return () =>
      cy.ompRetrievePayout(
        payoutId,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
  }

  context("Confirm / cancel / fulfill - cross-profile denial", () => {
    it("payout confirm blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompConfirmPayout(
            payoutIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { confirm: true },
            globalState
          ),
      });
    });

    it("payout cancel blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompCancelPayout(
            payoutIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            {},
            globalState
          ),
      });
    });

    it("payout fulfill blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompFulfillPayout(
            payoutIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            {},
            globalState
          ),
      });
    });
  });

  context("Inconsistent merchant_id / profile_id in request body", () => {
    it("payout confirm rejects inconsistent merchant_id", () => {
      cy.ompConfirmPayout(
        payoutIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { confirm: true, merchant_id: merchantIdM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("payout confirm rejects inconsistent profile_id", () => {
      cy.ompConfirmPayout(
        payoutIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { confirm: true, profile_id: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
