import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Auto Retry Tests", () => {
  before(() => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after(() => {
    cy.task("setGlobalState", globalState.data);
  });
  it("create secondary connector for the same profile", () => {
    const CONNECTOR_POOL = ["stripe", "adyen", "cybersource"];
    const primaryConnector = globalState.get("connectorId"); // Save Stripe here

    const secondaryConnector = CONNECTOR_POOL.find(
      (connector) => connector !== primaryConnector
    );

    globalState.set("connectorId", secondaryConnector);
    globalState.set("secondaryConnector", secondaryConnector);

    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      payment_methods_enabled,
      globalState,
      "profile",
      "merchantConnectorSecondary"
    ).then(() => {

      globalState.set("connectorId", primaryConnector);
    });
  });

  context("auto retries enabled with max retries = 1", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.AUTO_RETRY
        )
      ) {
        cy.log(
          `Skipping Auto Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("updates business profile to enable auto retries with 1 max retry", () => {
      const updateBusinessProfileBody = {
        is_auto_retries_enabled: true,
        max_auto_retries_enabled: 1,
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

      globalState.set("max_auto_retries_enabled", 1);
    });

    it("creates a payment intent", () => {
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

    it("confirm payment", () => {
      const activeConnector = globalState.get("connectorId");

      const data =
        getConnectorDetails(activeConnector)["card_pm"]["No3DSFailPayment"];

      cy.confirmCallAutoRetryTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
    });

    it("retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.retrievePaymentCallAutoRetryTest({ globalState, data });
    });
  });

  context("auto retries enabled with max retries = 0", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      const connectorId = globalState.get("connectorId");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.AUTO_RETRY
        )
      ) {
        cy.log(
          `Skipping Auto Retry - connector not supported: ${connectorId}`
        );
        this.skip();
      }
    });

    it("updates business profile to enable auto retries with 0 max retries", () => {
      const body = {
        is_auto_retries_enabled: true,
        max_auto_retries_enabled: 0,
      };

      cy.UpdateBusinessProfileTest(
        body,
        false,
        false,
        false,
        false,
        false,
        globalState
      );

      // Sync local state so the command knows to expect failure
      globalState.set("max_auto_retries_enabled", 0);
    });

    it("create payment intent", () => {
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

    it("confirm payment", () => {
      const activeConnector = globalState.get("connectorId");

      const data =
        getConnectorDetails(activeConnector)["card_pm"]["No3DSFailPayment"];

      cy.confirmCallAutoRetryTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
    });
    it("retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.retrievePaymentCallAutoRetryTest({ globalState, data });
    });
  });
});
