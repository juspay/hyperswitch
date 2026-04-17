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
    "Card-NoThreeDS payment with billing_descriptor Create+Confirm",
    () => {
      it("Create and Confirm Payment with billing_descriptor -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment with billing_descriptor", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["BillingDescriptorNo3DSAutoCapture"];

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

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["BillingDescriptorNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );
});
