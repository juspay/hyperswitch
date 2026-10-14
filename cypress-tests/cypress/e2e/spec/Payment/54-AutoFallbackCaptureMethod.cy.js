import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
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

      it("confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["AutoFallbackDisabled"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["AutoFallbackDisabled"];

        cy.retrievePaymentCallTest({ globalState, data });
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
        ]["AutoFallbackCaptureMethod"]["AutoFallbackEnabled"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["AutoFallbackCaptureMethod"]["AutoFallbackEnabled"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );
});
