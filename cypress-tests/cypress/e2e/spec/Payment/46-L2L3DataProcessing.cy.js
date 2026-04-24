import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("L2/L3 Data Processing Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("L2/L3 Data - Intent+Confirm Flow", () => {
    it("Update Business Profile (L2/L3 enabled) -> Create Payment Intent -> Payment Methods Call -> Confirm Payment with L2/L3 -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile (L2/L3 enabled)", () => {
        cy.businessProfileUpdate(
          fixtures.businessProfile.bpUpdate,
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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with L2/L3 Data", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with L2/L3 Data");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("L2/L3 Data - Negative Test (Feature Disabled)", () => {
    it("Attempt L2/L3 payment WITHOUT is_l2_l3_enabled -> Should process without L2/L3 data", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile (L2/L3 disabled)", () => {
        // Create a profile update payload with L2/L3 disabled
        const bpDisabled = {
          is_connector_agnostic_mit_enabled: true,
          collect_shipping_details_from_wallet_connector: true,
          collect_billing_details_from_wallet_connector: true,
          always_collect_billing_details_from_wallet_connector: true,
          always_collect_shipping_details_from_wallet_connector: true,
          is_l2_l3_enabled: false,
        };
        cy.businessProfileUpdate(bpDisabled, globalState);
      });

      cy.step("Create and Confirm Payment with L2/L3 fields", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment and verify", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
