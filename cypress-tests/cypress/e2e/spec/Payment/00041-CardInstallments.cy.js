import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("Card - Installment payment flow test", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.CARD_INSTALLMENTS
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card installment payment - Create and Confirm", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-and-confirm-installment-payment", () => {
      const createData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithInstallments"];

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CardInstallmentConfirm"];

      cy.createPaymentIntentTest(
        Cypress._.cloneDeep(fixtures.createPaymentBody),
        createData,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.confirmCallTest(
        Cypress._.cloneDeep(fixtures.confirmBody),
        confirmData,
        true,
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CardInstallmentConfirm"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
  context(
    "Card installment payment - Create+Confirm with installment options should fail",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create+confirm-installment-options-should-error", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentWithInstallmentsAndConfirmTrue"];

        cy.createPaymentIntentTest(
          Cypress._.cloneDeep(fixtures.createPaymentBody),
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );
});
