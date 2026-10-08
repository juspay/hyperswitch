import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { cardCreditDebitEnabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Auto Fallback Capture Method flow test", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = globalState.get("connectorId");
      specShouldSkip = utils.shouldIncludeConnector(
        connectorId,
        utils.CONNECTOR_LISTS.INCLUDE.AUTO_FALLBACK_CAPTURE_METHOD
      );
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "setup merchant, api key, connector and customer for auto fallback tests",
    () => {
      it("merchant-create-call-test", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditDebitEnabled,
          globalState
        );
      });

      it("create-customer-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });
    }
  );

  context(
    "auto_fallback_capture_method disabled - manual capture confirm is rejected",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("update business profile with auto_fallback_capture_method disabled", () => {
        const updateBusinessProfileBody = {
          auto_fallback_capture_method: false,
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PaymentIntent"];

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

      it("confirm-payment-call-test with no eligible connector error", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["ConfirmDisabled"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    }
  );

  context(
    "auto_fallback_capture_method enabled - manual capture falls back to automatic",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("update business profile with auto_fallback_capture_method enabled", () => {
        const updateBusinessProfileBody = {
          auto_fallback_capture_method: true,
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PaymentIntent"];

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

      it("confirm-payment-call-test with capture_method_applied automatic", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["ConfirmEnabled"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "auto_fallback_capture_method toggles payment method list visibility",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("update business profile with auto_fallback_capture_method enabled", () => {
        const updateBusinessProfileBody = {
          auto_fallback_capture_method: true,
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PaymentIntent"];

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

      it("payment-method-list-call-test with card visible", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PmListEnabled"];

        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });

      it("update business profile with auto_fallback_capture_method disabled", () => {
        const updateBusinessProfileBody = {
          auto_fallback_capture_method: false,
        };
        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PaymentIntent"];

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

      it("payment-method-list-call-test with card hidden", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["PmListDisabled"];

        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );
});
