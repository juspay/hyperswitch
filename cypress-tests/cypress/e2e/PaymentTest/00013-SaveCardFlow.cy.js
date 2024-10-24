import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

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
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!should_continue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["ResponseCustom"] ? data["ResponseCustom"] : data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(
          saveCardBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Save card for NoThreeDS manual full capture payment- Create+Confirm [on_session]",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!should_continue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["ResponseCustom"] ? data["ResponseCustom"] : data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(
          saveCardBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Save card for NoThreeDS manual partial capture payment- Create + Confirm [on_session]",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!should_continue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["ResponseCustom"] ? data["ResponseCustom"] : data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(
          saveCardBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          100,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Save card for NoThreeDS automatic capture payment [off_session]",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!should_continue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];
        let req_data = data["Request"];
        let res_data = data["ResponseCustom"] ? data["ResponseCustom"] : data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmAutoCaptureOffSession"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.saveCardConfirmCallTest(
          saveCardBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Save card for NoThreeDS manual capture payment- Create+Confirm [off_session]",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSManualCaptureOffSession"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });
      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardConfirmManualCaptureOffSession"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(
          saveCardBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });
    }
  );
});
