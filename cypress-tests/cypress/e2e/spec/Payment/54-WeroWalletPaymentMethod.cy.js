import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

// PR #14658 adds `wero` as a recognized wallet payment_method_type/WalletData variant
// across the request schema and every connector's exhaustive match arms. No connector
// actually implements the Wero gateway integration yet (every arm this PR touches maps
// it to a generic "not implemented for this connector" case), so there's nothing to
// assert about a real payment going through. What's actually testable from outside is
// the request-shape regression this PR fixes: before it, `wero_redirect` wasn't a known
// WalletData variant at all, so any payment with it failed at JSON deserialization
// before reaching routing or the connector. After it, deserialization succeeds and the
// request fails later (routing or the connector), with a normal API error instead of a
// raw deserialize error.
describe("Wero wallet payment_method_data is accepted as a known request shape", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Wallet - wero_redirect payment_method_data", () => {
    it("create-payment-with-wero-wallet", () => {
      const body = {
        ...fixtures.createConfirmPaymentBody,
        amount: 1000,
        currency: "EUR",
        confirm: true,
        payment_method: "wallet",
        payment_method_type: "wero",
        payment_method_data: {
          wallet: {
            wero_redirect: {},
          },
        },
        profile_id: globalState.get("profileId"),
        customer_id: globalState.get("customerId"),
      };

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
        body,
      }).then((response) => {
        // The one thing this PR actually changes: no "Json deserialize error:
        // unknown variant `wero_redirect`" — whatever happens next (routing finds no
        // connector, or a connector rejects it as not implemented) is a separate,
        // pre-existing concern this test doesn't claim to cover.
        const errorMessage = response.body?.error?.message;
        if (errorMessage) {
          expect(errorMessage, "error message").to.not.include(
            "unknown variant"
          );
        }
      });
    });
  });
});
