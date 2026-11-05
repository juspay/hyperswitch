import State from "../../../../utils/State";

// render_payout_link uses auth::MerchantIdAuth(merchant_id) - the path's
// own merchant_id IS the authenticated identity, there's no separate
// bearer/api-key credential on this public, customer-facing route at all.
// The core resolves the payout and payout attempt via merchant-scoped
// queries (find_payout_by_merchant_id_payout_id,
// find_payout_attempt_by_merchant_id_payout_id_payout_attempt_id) before
// resolving the payout_link_id from the payout's own record, not from
// attacker input - so the cross-merchant check here is entirely "does a
// path merchant_id that doesn't own this payout_id still resolve," which
// should be denied.
//
// Payout Link is hard-restricted to the "wise" connector in this repo's
// own CONNECTOR_LISTS.INCLUDE.PAYOUT_LINK (configs/Payout/Utils.js) -
// gated on the same payoutLinkExecution flag 00-Setup.cy.js sets.
//
// Route (verified): GET /payout_link/{merchant_id}/{payout_id}.
let globalState;
let merchantIdM1;
let merchantIdM2;
let payoutLinkPayoutId;
let payoutLinkExecution;

describe("OMP Scoping - Payout Link", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      payoutLinkPayoutId = globalState.get("payoutLinkPayoutId");
      payoutLinkExecution = globalState.get("payoutLinkExecution");

      if (!payoutLinkExecution || !payoutLinkPayoutId) {
        cy.task(
          "cli_log",
          "Skipping this suite: payout links are only supported by the " +
            "wise connector in this repo, not the connector this suite's " +
            "payouts actually use. See the PAYOUT_LINK_PA step in " +
            "00-Setup.cy.js."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!payoutLinkExecution || !payoutLinkPayoutId) {
      this.skip();
    }
  });

  it("M2's path merchant_id cannot reach M1's payout_id - denied", () => {
    cy.ompRetrievePayoutLinkByPath(
      merchantIdM2,
      payoutLinkPayoutId,
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("M1's path merchant_id reaches its own payout_id (positive)", () => {
    cy.ompRetrievePayoutLinkByPath(
      merchantIdM1,
      payoutLinkPayoutId,
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("a non-existent payout_id under M1's path returns 404", () => {
    cy.ompRetrievePayoutLinkByPath(
      merchantIdM1,
      "payout_nonexistent_omp_test",
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });
});
