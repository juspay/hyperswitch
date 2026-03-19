import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Installment payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card installment payment - Adyen configured + USD currency → installment_options: null",
    () => {
      it("Create Payment Intent -> Payment Methods Call", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with USD", () => {
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
          "Payment Methods Call - Verify installment_options is null",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Payment Methods Call");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "pm_list"
            ]["PmListResponse"]["PmListWithInstallmentsNull"];
            cy.paymentMethodsCallTest(globalState, data);
          }
        );
      });
    }
  );

  context(
    "Card installment payment - Adyen configured + BRL currency → installment_options populated",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with BRL and installments", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentWithInstallments"];

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

        cy.step("Payment Methods Call - Verify installment_options", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "pm_list"
          ]["PmListResponse"]["PmListWithInstallmentsBRL"];
          cy.paymentMethodsCallTest(globalState, data);
        });

        cy.step("Confirm Installment Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["CardInstallmentConfirm"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

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
          ]["CardInstallmentConfirm"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card installment payment - Create with confirm true should fail",
    () => {
      it("Create+Confirm Payment with installments should error", () => {
        cy.step(
          "Create Payment Intent with confirm true and installments",
          () => {
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["PaymentIntentWithInstallmentsAndConfirmTrue"];

            cy.createPaymentIntentTest(
              fixtures.createPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );
          }
        );
      });
    }
  );
});
