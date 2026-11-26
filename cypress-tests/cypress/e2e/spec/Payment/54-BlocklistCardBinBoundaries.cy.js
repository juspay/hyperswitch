import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

const blocklistContext = () => ({
  processor_merchant_id: globalState.get("merchantId"),
  provider_merchant_id: globalState.get("merchantId"),
});

describe("Blocklist card_bin / extended_card_bin / generic_card_bin boundaries", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping blocklist spec"
        );
        specShouldSkip = true;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
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

  context("card_bin - unchanged 6-digit-exact validation", () => {
    it("should accept a 6 digit card_bin (regression)", () => {
      cy.blocklistCreateRule({}, "424242", globalState, "card_bin");
    });

    it("cleanup: delete the 6 digit card_bin entry", () => {
      cy.blocklistDeleteRule("card_bin", "424242", globalState);
    });

    it("should reject a 5 digit card_bin", () => {
      cy.blocklistCreateRule({}, "42424", globalState, "card_bin", false);
    });

    it("should reject a 7 digit card_bin (still not widened)", () => {
      cy.blocklistCreateRule({}, "4242424", globalState, "card_bin", false);
    });
  });

  context(
    "extended_card_bin - unchanged 8-digit-exact validation (deprecated, kept for backward compat)",
    () => {
      it("should still accept an 8 digit extended_card_bin", () => {
        cy.blocklistCreateRule(
          {},
          "42424242",
          globalState,
          "extended_card_bin"
        );
      });

      it("cleanup: delete the extended_card_bin entry", () => {
        cy.blocklistDeleteRule("extended_card_bin", "42424242", globalState);
      });

      it("should reject a 6 digit extended_card_bin", () => {
        cy.blocklistCreateRule(
          {},
          "424242",
          globalState,
          "extended_card_bin",
          false
        );
      });
    }
  );

  context("generic_card_bin - new 6 to 10 digit range", () => {
    it("should accept a 6 digit generic_card_bin", () => {
      cy.blocklistCreateRule({}, "424242", globalState, "generic_card_bin");
    });

    it("cleanup: delete the 6 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRule("generic_card_bin", "424242", globalState);
    });

    it("should accept an 8 digit generic_card_bin (previously only valid as extended_card_bin)", () => {
      cy.blocklistCreateRule({}, "42424242", globalState, "generic_card_bin");
    });

    it("cleanup: delete the 8 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRule("generic_card_bin", "42424242", globalState);
    });

    it("should accept a 10 digit generic_card_bin (new upper bound)", () => {
      cy.blocklistCreateRule({}, "4242424242", globalState, "generic_card_bin");
    });

    it("cleanup: delete the 10 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRule("generic_card_bin", "4242424242", globalState);
    });

    it("should reject a 5 digit generic_card_bin (below minimum)", () => {
      cy.blocklistCreateRule(
        {},
        "42424",
        globalState,
        "generic_card_bin",
        false
      );
    });

    it("should reject an 11 digit generic_card_bin (above maximum)", () => {
      cy.blocklistCreateRule(
        {},
        "42424242424",
        globalState,
        "generic_card_bin",
        false
      );
    });
  });

  context("Eligibility check with an 8 digit generic_card_bin block", () => {
    // Deliberately a bin untouched by the boundary tests above (which all
    // churn the "424242" family via rapid create/delete cycles). Reusing
    // "42424242" here raced a blocklist cache that hadn't picked up the
    // fresh create yet, causing a consistent (not flaky) false "allow".
    it("should create a generic_card_bin blocklist rule for 40000000", () => {
      cy.blocklistCreateRule({}, "40000000", globalState, "generic_card_bin");
    });

    it("should enable blocklist functionality using Superposition", () => {
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        true,
        blocklistContext()
      );
    });

    it("should deny payment for a card matching the blocked 8 digit generic_card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.BlockedGenericCardBinDenied,
        globalState
      );
    });

    it("should allow payment for a non-blocklisted card", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.NonBlocklistedCardAllowed,
        globalState
      );
    });

    it("cleanup: delete the generic_card_bin rule", () => {
      cy.blocklistDeleteRule("generic_card_bin", "40000000", globalState);
    });

    it("should disable blocklist functionality using Superposition", () => {
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        false,
        blocklistContext()
      );
    });
  });
});
