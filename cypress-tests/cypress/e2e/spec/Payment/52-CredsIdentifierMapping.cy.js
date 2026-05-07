import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("MCA Credentials Identifier Mapping Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Auto Capture Flow - Payment with creds_identifier and subsequent refund",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("Create Payment Intent with creds_identifier", () => {
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

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm Payment Intent with creds_identifier", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CredsIdentifierMapping"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Create Refund with creds_identifier", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Retrieve Payment and verify refund attached", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );

  context(
    "Manual Capture Flow - Payment with creds_identifier, capture, and refund",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("Create Manual Capture Payment Intent with creds_identifier", () => {
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

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm Manual Capture Payment Intent with creds_identifier", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CredsIdentifierMapping"];

        const manualCaptureData = {
          ...data,
          Response: {
            status: 200,
            body: {
              status: "requires_capture",
            },
          },
        };

        cy.confirmCallTest(
          fixtures.confirmBody,
          manualCaptureData,
          true,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(manualCaptureData);
      });

      it("Capture Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Create Refund for Manual Capture Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("Retrieve Payment after Refund", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );
});
