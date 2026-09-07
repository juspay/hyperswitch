import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

describe("Blocklist card_bin / extended_card_bin / generic_card_bin boundaries", () => {
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

  context("card_bin - unchanged 6-digit-exact validation", () => {
    it("should accept a 6 digit card_bin (regression)", () => {
      cy.blocklistCreateRuleRaw("card_bin", "424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property("data_kind", "card_bin");
          expect(response.body).to.have.property(
            "fingerprint_id",
            "424242"
          );
        }
      );
    });

    it("cleanup: delete the 6 digit card_bin entry", () => {
      cy.blocklistDeleteRuleRaw("card_bin", "424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should reject a 5 digit card_bin", () => {
      cy.blocklistCreateRuleRaw("card_bin", "42424", globalState).then(
        (response) => {
          expect(response.status).to.not.equal(200);
        }
      );
    });

    it("should reject a 7 digit card_bin (still not widened)", () => {
      cy.blocklistCreateRuleRaw("card_bin", "4242424", globalState).then(
        (response) => {
          expect(response.status).to.not.equal(200);
        }
      );
    });
  });

  context("extended_card_bin - unchanged 8-digit-exact validation (deprecated, kept for backward compat)", () => {
    it("should still accept an 8 digit extended_card_bin", () => {
      cy.blocklistCreateRuleRaw("extended_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "data_kind",
            "extended_card_bin"
          );
        }
      );
    });

    it("cleanup: delete the extended_card_bin entry", () => {
      cy.blocklistDeleteRuleRaw("extended_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should reject a 6 digit extended_card_bin", () => {
      cy.blocklistCreateRuleRaw("extended_card_bin", "424242", globalState).then(
        (response) => {
          expect(response.status).to.not.equal(200);
        }
      );
    });
  });

  context("generic_card_bin - new 6 to 10 digit range", () => {
    it("should accept a 6 digit generic_card_bin", () => {
      cy.blocklistCreateRuleRaw("generic_card_bin", "424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "data_kind",
            "generic_card_bin"
          );
        }
      );
    });

    it("cleanup: delete the 6 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRuleRaw("generic_card_bin", "424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should accept an 8 digit generic_card_bin (previously only valid as extended_card_bin)", () => {
      cy.blocklistCreateRuleRaw("generic_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "data_kind",
            "generic_card_bin"
          );
        }
      );
    });

    it("cleanup: delete the 8 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRuleRaw("generic_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should accept a 10 digit generic_card_bin (new upper bound)", () => {
      cy.blocklistCreateRuleRaw(
        "generic_card_bin",
        "4242424242",
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property(
          "data_kind",
          "generic_card_bin"
        );
      });
    });

    it("cleanup: delete the 10 digit generic_card_bin entry", () => {
      cy.blocklistDeleteRuleRaw("generic_card_bin", "4242424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should reject a 5 digit generic_card_bin (below minimum)", () => {
      cy.blocklistCreateRuleRaw("generic_card_bin", "42424", globalState).then(
        (response) => {
          expect(response.status).to.not.equal(200);
        }
      );
    });

    it("should reject an 11 digit generic_card_bin (above maximum)", () => {
      cy.blocklistCreateRuleRaw(
        "generic_card_bin",
        "42424242424",
        globalState
      ).then((response) => {
        expect(response.status).to.not.equal(200);
      });
    });
  });

  context("Eligibility check with an 8 digit generic_card_bin block", () => {
    it("should create a generic_card_bin blocklist rule for 42424242", () => {
      cy.blocklistCreateRuleRaw("generic_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should enable blocklist functionality using configs API", () => {
      const merchantId = globalState.get("merchantId");
      const key = `guard_blocklist_for_${merchantId}`;
      cy.setConfigs(globalState, key, "true", "CREATE");
    });

    it("should deny payment for a card matching the blocked 8 digit generic_card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.BlocklistedCardDenied,
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
      cy.blocklistDeleteRuleRaw("generic_card_bin", "42424242", globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
        }
      );
    });

    it("should disable blocklist functionality using configs API", () => {
      const merchantId = globalState.get("merchantId");
      const key = `guard_blocklist_for_${merchantId}`;
      cy.setConfigs(globalState, key, "true", "DELETE");
    });
  });
});
