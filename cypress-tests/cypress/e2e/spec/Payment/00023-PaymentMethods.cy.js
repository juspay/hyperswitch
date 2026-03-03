import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create payment method for customer", () => {
    it("Create Customer -> Create Payment Method -> List PM for Customer", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.step("Create Payment Method", () =>
        cy.createPaymentMethodTest(globalState, data)
      );

      cy.step("List PM for Customer", () =>
        cy.listCustomerPMCallTest(globalState)
      );
    });
  });

  context("Set default payment method", () => {
    it("List PM for Customer -> Create Payment Method -> Create Payment Intent -> Confirm Payment -> List PM for Customer -> Set Default Payment Method", () => {
      cy.step("List PM for Customer", () =>
        cy.listCustomerPMCallTest(globalState)
      );

      const pmData = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.step("Create Payment Method", () =>
        cy.createPaymentMethodTest(globalState, pmData)
      );

      const intentData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          intentData,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(intentData)) return;

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];

      cy.step("Confirm Payment", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("List PM for Customer", () =>
        cy.listCustomerPMCallTest(globalState)
      );

      cy.step("Set Default Payment Method", () =>
        cy.setDefaultPaymentMethodTest(globalState)
      );
    });
  });

  context("Delete payment method for customer", () => {
    it("Create Customer -> Create Payment Method -> List PM for Customer -> Delete Payment Method for Customer", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.step("Create Payment Method", () =>
        cy.createPaymentMethodTest(globalState, data)
      );

      cy.step("List PM for Customer", () =>
        cy.listCustomerPMCallTest(globalState)
      );

      cy.step("Delete Payment Method for Customer", () =>
        cy.deletePaymentMethodTest(globalState)
      );
    });
  });

  context("'Last Used' off-session token payments", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    context("No 3DS save card", () => {
      it("Create Customer -> Create+Confirm Payment (No 3DS Off Session) -> List PM for Customer", () => {
        cy.step("Create Customer", () =>
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
        );

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.step("Create+Confirm Payment (No 3DS Off Session)", () =>
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);

        cy.step("List PM for Customer", () =>
          cy.listCustomerPMCallTest(globalState)
        );
      });
    });

    context("3DS save card", () => {
      it("Create+Confirm Payment (3DS Off Session) -> Handle Redirection -> List PM for Customer", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUse3DSAutoCaptureOffSession"];

        cy.step("Create+Confirm Payment (3DS Off Session)", () =>
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
        if (!shouldContinue) return;

        const expectedRedirection = fixtures.confirmBody["return_url"];
        cy.step("Handle Redirection", () =>
          cy.handleRedirection(globalState, expectedRedirection)
        );

        cy.step("List PM for Customer", () =>
          cy.listCustomerPMCallTest(globalState)
        );
      });
    });

    context("3DS save card with token", () => {
      it("Create Payment Intent -> Confirm Save Card Payment -> Handle Redirection -> List PM for Customer", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

        const intentData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            intentData,
            "three_ds",
            "automatic",
            globalState
          )
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(intentData);
        if (!shouldContinue) return;

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        const newData = {
          ...confirmData,
          Response: {
            ...confirmData.Response,
            body: {
              ...confirmData.Response.body,
              status: "requires_customer_action",
            },
          },
        };

        cy.step("Confirm Save Card Payment", () =>
          cy.saveCardConfirmCallTest(saveCardBody, newData, globalState)
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(confirmData);
        if (!shouldContinue) return;

        const expectedRedirection = fixtures.confirmBody["return_url"];
        cy.step("Handle Redirection", () =>
          cy.handleRedirection(globalState, expectedRedirection)
        );

        cy.step("List PM for Customer", () =>
          cy.listCustomerPMCallTest(globalState, 1)
        );
      });
    });

    context("No 3DS save card with token", () => {
      it("Create Payment Intent -> Confirm Save Card Payment -> List PM for Customer", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

        const intentData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            intentData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(intentData);
        if (!shouldContinue) return;

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCapture"];

        cy.step("Confirm Save Card Payment", () =>
          cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState)
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(confirmData);

        cy.step("List PM for Customer", () =>
          cy.listCustomerPMCallTest(globalState)
        );
      });
    });
  });
});
