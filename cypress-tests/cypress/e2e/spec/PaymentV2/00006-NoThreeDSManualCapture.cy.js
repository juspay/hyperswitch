import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - NoThreeDS Manual Full Capture payment flow test", () => {
    context("payment Create and Confirm", () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("confirm-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.retrievePaymentCallTest(globalState, data);
      });
    });

    context("Payment Create+Confirm", () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.retrievePaymentCallTest(globalState, data);
      });
    });
  });

  context(
    "Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm",
    () => {
      context("payment Create and Payment Confirm", () => {
        let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("create-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("payment_methods-call-test", () => {
          cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest(globalState, data);
        });

        it("capture-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          cy.retrievePaymentCallTest(globalState, data);
        });
      });

      context("payment + Confirm", () => {
        let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("create+confirm-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest(globalState, data);
        });

        it("capture-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("retrieve-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          cy.retrievePaymentCallTest(globalState, data);
        });
      });
    }
  );
});
