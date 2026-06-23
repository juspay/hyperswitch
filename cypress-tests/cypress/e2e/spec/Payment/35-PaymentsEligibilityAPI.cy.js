import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

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
        connectorDetails.eligibility_api.PaymentIntentForBlocklist,
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
        connectorDetails.eligibility_api.BlocklistedCardDenied,
        globalState
      );
    });

    it("should allow payment for non-blocklisted card", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.NonBlocklistedCardAllowed,
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
