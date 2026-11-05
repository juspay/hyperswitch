import * as fixtures from "../../../../fixtures/imports";
import State from "../../../../utils/State";
import { CardPayout } from "../../../configs/Security/Commons";

// PO_PA / PO_PB are payouts under M1, one per profile, created in
// 00-Setup.cy.js (skipped there if the configured payout connector has no
// credentials - gate on the same payoutsExecution flag here, matching the
// existing Payout/*.cy.js convention in this repo).
//
// update/confirm/cancel/fulfill are ApiKeyAuth-only routes (verified against
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
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!payoutsExecution) {
      cy.task(
        "cli_log",
        "Skipping: no payout_processor credentials in the creds file (see 00-Setup.cy.js)."
      );
      this.skip();
    }
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

  context("Update / confirm / cancel / fulfill - cross-profile denial", () => {
    it("payout update blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompUpdatePayout(
            payoutIdPB,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { metadata: { attempted_by: "PA" } },
            globalState
          ),
      });
    });

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

    it("PA updates its own payout (positive)", () => {
      // PO_PA itself is already in requires_fulfillment (confirm=true,
      // auto_fulfill=false at creation in 00-Setup.cy.js) - update is
      // blocked there by a hard status gate before any field is even
      // inspected (is_payout_initiated/is_payout_terminal_state in
      // crates/router/src/core/payouts/helpers.rs), and fulfilling it only
      // moves it to a terminal state, which is also blocked. The only
      // status update is ever permitted on is pre-confirm, so this test
      // creates its own throwaway unconfirmed payout rather than reusing
      // PO_PA, which can never be updated again once created.
      cy.createConfirmPayoutTest(
        structuredClone(fixtures.createPayoutBody),
        {
          ...CardPayout(),
          Request: {
            ...CardPayout().Request,
            profile_id: profileIdPA,
          },
        },
        false,
        false,
        globalState
      );

      cy.then(() => {
        const throwawayPayoutId = globalState.get("payoutID");

        cy.ompUpdatePayout(
          throwawayPayoutId,
          { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
          { metadata: { attempted_by: "PA" } },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(200);
        });
      });
    });
  });

  context("Inconsistent merchant_id / profile_id in request body", () => {
    it("payout update rejects inconsistent merchant_id", () => {
      cy.ompUpdatePayout(
        payoutIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { merchant_id: merchantIdM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

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

    it("payout update rejects inconsistent profile_id", () => {
      cy.ompUpdatePayout(
        payoutIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { profile_id: profileIdPB },
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

  context("Edge cases", () => {
    it("non-existent payout ID returns 404", () => {
      cy.ompUpdatePayout(
        "payout_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { metadata: {} },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("malformed profile_id in payout body is rejected, not 500", () => {
      cy.ompUpdatePayout(
        payoutIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { profile_id: "!!not-a-valid-id!!" },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid credential on payout update is rejected", () => {
      cy.ompUpdatePayout(
        payoutIdPA,
        { type: "apiKey", apiKey: "not_a_real_api_key" },
        { metadata: {} },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });

    it("revoked API key is rejected on payout update", () => {
      // Creating/revoking an API key needs the admin API key, not a
      // merchant's own api-key (see getManagementAuthHeaders in
      // commands.js) - the merchant key is only valid for routes that
      // operate as that merchant, and key management isn't one of them.
      const merchantIdM1 = globalState.get("merchantIdM1");

      cy.ompCreateThrowawayApiKey(merchantIdM1, globalState).then(
        ({ apiKey: throwawayKey, keyId }) => {
          cy.ompRevokeApiKey(merchantIdM1, keyId, globalState).then(() => {
            cy.ompUpdatePayout(
              payoutIdPA,
              { type: "apiKey", apiKey: throwawayKey },
              { metadata: {} },
              globalState
            ).then((res) => {
              expect(res.status).to.eq(401);
            });
          });
        }
      );
    });
  });
});
