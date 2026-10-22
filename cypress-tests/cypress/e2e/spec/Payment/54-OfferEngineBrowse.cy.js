import State from "../../../utils/State";

let globalState;

// PR #13999: POST /offer_engine/offers/list — a merchant-dashboard browse
// endpoint, separate from the payment-flow eligibility endpoint covered by
// 54-OfferEngine.cy.js. No payment intent is involved; it just surfaces
// whatever offers this merchant currently has configured and eligible.
describe("Offer Engine - Browse Offers", () => {
  before("seed global state and verify Offer Engine connectivity", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        return cy.offerEngineConnectivityCheck(globalState);
      })
      .then((reachable) => {
        if (reachable) {
          return cy.wrap(true);
        }

        return cy.offerEngineMerchantConfiguredCheck(globalState);
      })
      .then((reachable) => {
        if (!reachable) {
          cy.task(
            "cli_log",
            "Offer Engine is not reachable/enabled in this environment, skipping Offer Engine Browse spec"
          );
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("browse offers without an order context", () => {
    cy.browseOffersCall(
      {},
      {
        Response: {
          status: 200,
        },
      },
      globalState
    );
  });

  it("browse offers with an order context", () => {
    cy.browseOffersCall(
      {
        offer_payment_info: {
          currency: "USD",
        },
      },
      {
        Response: {
          status: 200,
        },
      },
      globalState
    );
  });
});
