import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("BitPay - Crypto Refund flow - Not Implemented", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "BitPay - Refund flow should return NotImplemented error",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment (expect NotImplemented)", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "crypto_pm"
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

        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "crypto_pm"
          ]["CryptoCurrency"];
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

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "crypto_pm"
          ]["CryptoCurrency"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment (expect NotImplemented)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(globalState.get("connectorId"))[
            "crypto_pm"
          ]["Refund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        });
      });
    }
  );
});
