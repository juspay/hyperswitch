import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Billing Descriptor payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card-NoThreeDS payment with billing descriptor Create and confirm",
    () => {
      it("Create Payment Intent with billing_descriptor -> Confirm Payment -> Retrieve Payment and assert billing_descriptor persisted", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with billing_descriptor", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithBillingDescriptor"];

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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

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

        cy.step("Retrieve Payment and assert billing_descriptor", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment and assert billing_descriptor"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });
        });
      });
    }
  );
});
