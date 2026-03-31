import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Business Profile Payment Method Blocking", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card Issuing Country Blocking", () => {
    it("should block payment when card issuing country is blocked", () => {
      let shouldContinue = true;

      cy.step("Enable blocklist", () => {
        cy.blocklistToggle("true", globalState);
      });

      cy.step("Update business profile to block issuing country", () => {
        const updateBusinessProfileBody = {
          payment_method_blocking: {
            card: {
              issuing_country: ["US"],
            },
          },
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false,
          false,
          false,
          false,
          false,
          globalState
        );
      });

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent - should be blocked", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "payment_method_blocking_pm"
        ]["BlockIssuingCountry"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Disable blocklist", () => {
        cy.blocklistToggle("false", globalState);
      });
    });
  });

  context("Card Type Blocking", () => {
    it("should block payment when debit cards are blocked", () => {
      let shouldContinue = true;

      cy.step("Enable blocklist", () => {
        cy.blocklistToggle("true", globalState);
      });

      cy.step("Update business profile to block debit cards", () => {
        const updateBusinessProfileBody = {
          payment_method_blocking: {
            card: {
              card_types: ["debit"],
            },
          },
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false,
          false,
          false,
          false,
          false,
          globalState
        );
      });

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent - should be blocked", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "payment_method_blocking_pm"
        ]["BlockCardType"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Disable blocklist", () => {
        cy.blocklistToggle("false", globalState);
      });
    });
  });

  context("Card Subtype Blocking", () => {
    it("should block payment when card subtype is blocked", () => {
      let shouldContinue = true;

      cy.step("Enable blocklist", () => {
        cy.blocklistToggle("true", globalState);
      });

      cy.step("Update business profile to block specific card subtype", () => {
        const updateBusinessProfileBody = {
          payment_method_blocking: {
            card: {
              card_subtypes: ["smallcorporate"],
            },
          },
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false,
          false,
          false,
          false,
          false,
          globalState
        );
      });

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent - should be blocked", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "payment_method_blocking_pm"
        ]["BlockCardSubtype"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Disable blocklist", () => {
        cy.blocklistToggle("false", globalState);
      });
    });
  });

  context("Block If BIN Info Unavailable", () => {
    it("should block payment when BIN info is unavailable and block_if_bin_info_unavailable is true", () => {
      let shouldContinue = true;

      cy.step("Enable blocklist", () => {
        cy.blocklistToggle("true", globalState);
      });

      cy.step(
        "Update business profile to block when BIN info unavailable",
        () => {
          const updateBusinessProfileBody = {
            payment_method_blocking: {
              card: {
                block_if_bin_info_unavailable: true,
              },
            },
          };
          cy.UpdateBusinessProfileTest(
            updateBusinessProfileBody,
            false,
            false,
            false,
            false,
            false,
            globalState
          );
        }
      );

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Confirm Payment Intent - should be blocked when BIN unavailable",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }

          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["payment_method_blocking_pm"]["BlockIfBinInfoUnavailable"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
        }
      );

      cy.step("Disable blocklist", () => {
        cy.blocklistToggle("false", globalState);
      });
    });
  });
});
