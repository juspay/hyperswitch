import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] Billing Descriptor", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip if connector is not in the BILLING_DESCRIPTOR include list
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.BILLING_DESCRIPTOR
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] No3DS AutoCapture with Billing Descriptor", () => {
    let shouldContinue = true;

    it("create-payment-intent-with-billing-descriptor-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithBillingDescriptor"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-payment-with-billing-descriptor-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm Payment");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentConfirmWithBillingDescriptor"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-with-billing-descriptor-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentConfirmWithBillingDescriptor"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
});
