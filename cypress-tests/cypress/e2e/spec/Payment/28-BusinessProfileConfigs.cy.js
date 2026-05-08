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

  context("Outgoing Webhook Custom HTTP Headers", () => {
    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
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
        globalState
      );
    });

    it("Update business profile with new custom webhook headers and verify updated masked response", () => {
      const initialKeys = ["X-Custom-Header", "X-Short", "X-Tiny"];
      const webhookHeadersBody = {
        outgoing_webhook_custom_http_headers: {
          "X-Updated-Header": "updated-secret-value-long",
          "X-Another-Header": "another-long-value-string",
        },
      };
      cy.updateBusinessProfileWebhookCustomHeadersTest(
        webhookHeadersBody,
        globalState
      ).then(() => {
        // Verify old header keys from previous update are absent
        const responseHeaders = globalState.get("lastResponseHeaders") || {};
        initialKeys.forEach((key) => {
          expect(responseHeaders).to.not.have.property(key);
        });
      });
    });

    it("Clear custom webhook headers with empty object", () => {
      const webhookHeadersBody = {
        outgoing_webhook_custom_http_headers: {},
      };
      cy.updateBusinessProfileWebhookCustomHeadersTest(
        webhookHeadersBody,
        globalState
      );
    });

    // TODO: Add outgoing webhook delivery verification.
    // The reviewer requested verification that custom headers are actually
    // sent in outgoing webhooks. However, the Hyperswitch codebase does not
    // currently expose an API or mechanism to capture/intercept outgoing
    // webhook deliveries within Cypress tests.
    //
    // To implement this, we would need:
    // 1. A webhook event/events API that lists recent webhook deliveries
    // 2. Or an external webhook catcher service integrated into the test flow
    // 3. Or a way to configure a test endpoint that captures received headers
    //
    // Until such infrastructure exists, this verification is deferred.
  });
});
