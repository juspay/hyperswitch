import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let saveCardBody;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create payment method for customer", () => {
    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });
  });

  context("Set default payment method", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Set default payment method", () => {
      cy.setDefaultPaymentMethodTest(globalState);
    });
  });

  context("Delete payment method for customer", () => {
    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
      cy.createPaymentMethodTest(globalState, data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Delete Payment Method for a customer", () => {
      cy.deletePaymentMethodTest(globalState);
    });
  });

  context("'Last Used' off-session token payments", () => {
    let shouldContinue = true;

    beforeEach(function () {
      saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
      if (!shouldContinue) {
        this.skip();
      }
    });
    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    context("Create No 3DS off session save card payment", () => {
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

      it("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });
    });

    context("Create 3DS off session save card payment", () => {
      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUse3DSAutoCaptureOffSession"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle redirection", () => {
        const expectedRedirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expectedRedirection);
      });

      it("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });
    });

    context("Create 3DS off session save card payment with token", () => {
      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
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
          "three_ds",
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

        const newData = {
          ...data,
          Response: {
            ...data.Response,
            body: {
              ...data.Response.body,
              status: "requires_customer_action",
            },
          },
        };

        cy.saveCardConfirmCallTest(saveCardBody, newData, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle redirection", () => {
        const expectedRedirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expectedRedirection);
      });

      it("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState, 1 /* order */);
      });
    });

    context("Create No 3DS off session save card payment with token", () => {
      beforeEach(function () {
        saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        if (!shouldContinue) {
          this.skip();
        }
      });
      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
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

      it("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });
    });
  });
});
