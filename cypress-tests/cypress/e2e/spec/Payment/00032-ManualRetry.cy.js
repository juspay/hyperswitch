import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

const MANUAL_RETRY_EXPIRATION = 35000;

describe("Manual Retry Tests", () => {
  let globalState;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("manual-retry-disabled-test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.MANUAL_RETRY
        )
      ) {
        cy.log(
          `Skipping Manul Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("Update Profile with is_manual_retry_enabled", () => {
      const updateBusinessProfileBody = {
        is_manual_retry_enabled: false,
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

    it("First Confirm with Failed Status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Second Confirm with Error thrown", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ManualRetryPaymentDisabled"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("manual-retry-enabled-test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.MANUAL_RETRY
        )
      ) {
        cy.log(
          `Skipping Manul Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("Update Profile with is_manual_retry_enabled", () => {
      const updateBusinessProfileBody = {
        is_manual_retry_enabled: true,
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

    it("First Confirm with Failed Status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Retry Confirm with Successful Status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ManualRetryPaymentEnabled"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("manual-retry-cutoff-test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.MANUAL_RETRY
        )
      ) {
        cy.log(
          `Skipping Manul Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("Update Profile with is_manual_retry_enabled", () => {
      const updateBusinessProfileBody = {
        is_manual_retry_enabled: true,
        session_expiry: 60,
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

    it("First Confirm with Failed Status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Retry Confirm after cutoff is expired (Should Throw Error)", () => {
      // wait for 35 seconds
      // eslint-disable-next-line cypress/no-unnecessary-waiting
      cy.wait(MANUAL_RETRY_EXPIRATION);
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ManualRetryPaymentCutoffExpired"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("first-confirm-after-cutoff-test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.MANUAL_RETRY
        )
      ) {
        cy.log(
          `Skipping Manul Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("Update Profile with is_manual_retry_enabled", () => {
      const updateBusinessProfileBody = {
        is_manual_retry_enabled: true,
        session_expiry: 60,
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

    it("First Confirm after Manual Retry Cutoff (Should Succeed)", () => {
      // wait for 35 seconds
      // eslint-disable-next-line cypress/no-unnecessary-waiting
      cy.wait(MANUAL_RETRY_EXPIRATION);

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });
  });
});
