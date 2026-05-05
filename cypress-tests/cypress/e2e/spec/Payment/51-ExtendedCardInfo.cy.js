import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Extended Card Info Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Extended Card Info - Enable feature, make card payment, retrieve encrypted card data",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Set Extended Card Info config with RSA public key", () => {
        cy.setExtendedCardInfoConfigTest(globalState);
      });

      it("Enable Extended Card Info on business profile", () => {
        cy.toggleExtendedCardInfoTest(true, globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardInfo"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Extended Card Info - expect 200 with encrypted payload", () => {
        cy.retrieveExtendedCardInfoTest(200, globalState);
      });
    }
  );

  context(
    "Extended Card Info - Disable feature, confirm payment, verify info is not retrievable",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Disable Extended Card Info on business profile", () => {
        cy.toggleExtendedCardInfoTest(false, globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardInfo"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Extended Card Info - expect 404 (feature disabled, no data stored)", () => {
        cy.retrieveExtendedCardInfoTest(404, globalState);
      });
    }
  );

  context(
    "Extended Card BIN - Enable config, confirm payment, verify 8-digit BIN in response",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      after("Cleanup Extended Card BIN config", () => {
        if (globalState.get("extendedCardBinEnabled")) {
          cy.enableExtendedCardBinTest(false, globalState);
        }
      });

      it("Enable Extended Card BIN via configs API", () => {
        cy.enableExtendedCardBinTest(true, globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardInfo"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment and verify extended BIN (8 digits) is present", () => {
        cy.retrievePaymentAndVerifyExtendedBinTest(true, globalState);
      });
    }
  );

  context(
    "Extended Card BIN - Without config, confirm payment, verify extended BIN is absent",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardInfo"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment and verify extended BIN is absent", () => {
        cy.retrievePaymentAndVerifyExtendedBinTest(false, globalState);
      });
    }
  );
});
