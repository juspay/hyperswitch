import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let saveCardBody;

describe("Card - SaveCard payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Save card for NoThreeDS automatic capture payment- Create+Confirm [on_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue) {
          shouldContinue = utils.should_continue_further(data);
        }
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
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

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Save card for NoThreeDS manual full capture payment- Create+Confirm [on_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
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

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];

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
    }
  );

  context(
    "Save card for NoThreeDS manual partial capture payment- Create + Confirm [on_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
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

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];

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
    }
  );

  context(
    "Save card for NoThreeDS automatic capture payment [off_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Save card for NoThreeDS manual capture payment- Create+Confirm [off_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCaptureOffSession"];

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
        ]["SaveCardUseNo3DSManualCaptureOffSession"];

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

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmManualCaptureOffSession"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmManualCaptureOffSession"];

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
    }
  );

  context(
    "Save card for NoThreeDS automatic capture payment - create and confirm [off_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-save-card-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );
  context(
    "Use billing address from payment method during subsequent payment[off_session]",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-save-card-payment-call-test-without-billing", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSessionWithoutBilling"];

        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );
});
