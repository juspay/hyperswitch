import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Config Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Update collect_billing_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
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

      it("Update collect_billing_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          true, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  context(
    "Update collect_shipping_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Update collect_shipping_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  context(
    "Update always_collect_billing_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Update always_collect_billing_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          true, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  context(
    "Update always_collect_shipping_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Update always_collect_shipping_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          true, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  context(
    "Update always_collect_shipping_details_from_wallet_connector & collect_shipping_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Update both always & collect_shipping_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          true, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          true, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );
  context(
    "Update always_collect_billing_details_from_wallet_connector & to collect_billing_details_from_wallet_connector to true and verifying in payment method list, this config should be true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Update both always & collect_billing_details_from_wallet_connector to true", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          true, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          true, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  context(
    "Update all config(Collect address config) to false and verifying in payment method list, both config should be false",
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

      it("Update all config to false", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
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

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });
    }
  );

  // Connector-agnostic: webhook headers are Business Profile config, not connector-specific
  context("Outgoing Webhook Custom HTTP Headers", () => {
    const shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "webhookProfile"
      );
    });

    it("Update business profile with custom webhook headers and verify masked response", () => {
      const webhookHeadersBody = {
        outgoing_webhook_custom_http_headers: {
          "X-Custom-Header": "long-custom-value-six-chars",
          "X-Short": "secret",
          "X-Tiny": "xy",
        },
      };
      cy.updateBusinessProfileWebhookCustomHeadersTest(
        webhookHeadersBody,
        globalState,
        "webhookProfile"
      );
    });

    it("Update business profile with new custom webhook headers and verify updated masked response", () => {
      const previousHeaderKeys = Object.keys(
        globalState.get("lastResponseHeaders") ?? {}
      );
      const webhookHeadersBody = {
        outgoing_webhook_custom_http_headers: {
          "X-Updated-Header": "updated-secret-value-long",
          "X-Another-Header": "another-long-value-string",
        },
      };
      cy.updateBusinessProfileWebhookCustomHeadersTest(
        webhookHeadersBody,
        globalState,
        "webhookProfile",
        previousHeaderKeys
      );
    });

    it("Clear custom webhook headers with empty object", () => {
      const webhookHeadersBody = {
        outgoing_webhook_custom_http_headers: {},
      };
      cy.updateBusinessProfileWebhookCustomHeadersTest(
        webhookHeadersBody,
        globalState,
        "webhookProfile"
      );
    });
  });
});
