import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Billing Descriptor payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.BILLING_DESCRIPTOR
          )
        ) {
          skip = true;
          return;
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

  context("Card-NoThreeDS payment with billing descriptor", () => {
    it("Create Payment Intent -> Payment Methods -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with billing descriptor", () => {
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

      cy.step("Confirm Payment with billing descriptor", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithBillingDescriptor"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with billing descriptor", () => {
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

  context(
    "Card-NoThreeDS payment with invalid billing descriptor (field length validation)",
    () => {
      it("Create Payment Intent -> Payment Methods -> Confirm Payment (expect error)", () => {
        let shouldContinue = true;

        cy.step(
          "Create Payment Intent with invalid billing descriptor phone",
          () => {
            if (
              shouldIncludeConnector(
                connector,
                CONNECTOR_LISTS.INCLUDE.BILLING_DESCRIPTOR_INVALID_PHONE
              )
            ) {
              cy.log(
                `Skipping: ${connector} does not support billing descriptor field length validation test`
              );
              shouldContinue = false;
              return;
            }

            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["PaymentIntentWithBillingDescriptorInvalidPhone"];

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
          }
        );

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step(
          "Confirm Payment with invalid billing descriptor phone (expect error)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Confirm Payment");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["PaymentConfirmWithBillingDescriptorInvalidPhone"];

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          }
        );
      });
    }
  );
});
