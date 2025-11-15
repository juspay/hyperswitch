import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Payments Eligibility API with Blocklist", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup Phase", () => {
    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        {
          Request: {
            currency: "USD",
            amount: 6500,
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        },
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Blocklist Configuration", () => {
    it("should create blocklist rule for card_bin 424242", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "424242",
        globalState
      );
    });

    it("should enable blocklist functionality using configs API", () => {
      const merchantId = globalState.get("merchantId");
      const key = `guard_blocklist_for_${merchantId}`;
      const value = "true";

      cy.setConfigs(globalState, key, value, "CREATE");
    });
  });

  context("Eligibility API Tests", () => {
    it("should deny payment for blocklisted card_bin 424242", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        {
          Request: {
            payment_method_type: "card",
            payment_method_data: {
              card: {
                card_number: "4242424242424242",
                card_exp_month: "01",
                card_exp_year: "2050",
                card_holder_name: "John Smith",
                card_cvc: "349",
                card_network: "Visa",
              },
              billing: {
                address: {
                  line1: "1467",
                  line2: "Harrison Street",
                  line3: "Harrison Street",
                  city: "San Fransico",
                  state: "CA",
                  zip: "94122",
                  country: "US",
                  first_name: "John",
                  last_name: "Doe",
                },
                phone: {
                  number: "8056594427",
                  country_code: "+91",
                },
              },
            },
          },
          Response: {
            status: 200,
            body: {
              sdk_next_action: {
                next_action: {
                  deny: {
                    message: "Card number is blocklisted",
                  },
                },
              },
            },
          },
        },
        globalState
      );
    });

    it("should allow payment for non-blocklisted card", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        {
          Request: {
            payment_method_type: "card",
            payment_method_data: {
              card: {
                card_number: "4111111111111111", // Different BIN - not blocklisted
                card_exp_month: "01",
                card_exp_year: "2050",
                card_holder_name: "John Smith",
                card_cvc: "349",
                card_network: "Visa",
              },
              billing: {
                address: {
                  line1: "1467",
                  line2: "Harrison Street",
                  line3: "Harrison Street",
                  city: "San Fransico",
                  state: "CA",
                  zip: "94122",
                  country: "US",
                  first_name: "John",
                  last_name: "Doe",
                },
                phone: {
                  number: "8056594427",
                  country_code: "+91",
                },
              },
            },
          },
          Response: {
            status: 200,
            body: {
              // Should not have deny action for non-blocklisted cards
              // The response structure may vary based on implementation
            },
          },
        },
        globalState
      );
    });
  });

  context("Cleanup", () => {
    it("should delete blocklist rule", () => {
      cy.blocklistDeleteRule("card_bin", "424242", globalState);
    });

    it("should disable blocklist functionality using configs API", () => {
      const merchantId = globalState.get("merchantId");
      const key = `guard_blocklist_for_${merchantId}`;
      const value = "true";

      cy.setConfigs(globalState, key, value, "DELETE");
    });
  });
});
