import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let originalCustomerId;

describe("Config Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      originalCustomerId = globalState.get("customerId");
    });
  });

  after("flush global state", () => {
    // Some contexts in this spec create their own customers, overwriting
    // globalState.customerId. Restore the original customer before
    // flushing so later specs don't inherit one scoped to this spec's
    // own tests.
    globalState.set("customerId", originalCustomerId);
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

  context("Webhook Config Disabled Events — Create and Update", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Business Profile with webhook disabled events", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["WebhookConfig"]["Create"];
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        webhook_details: data.Request.webhook_details,
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookConfigProfile"
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Update Business Profile webhook disabled events", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["WebhookConfig"]["Update"];
      const updateBody = {
        webhook_details: data.Request.webhook_details,
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookConfigProfile"
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    after("cleanup webhookConfigProfile", () => {
      cy.deleteBusinessProfileTest(globalState, "webhookConfigProfile");
    });
  });

  context("Outgoing Webhook Event Gating — Status List Replacement", () => {
    beforeEach(function () {
      const connectorId = globalState.get("connectorId");
      const gatingConnectors =
        utils.CONNECTOR_LISTS.INCLUDE.OUTGOING_WEBHOOK_EVENT_CONFIG;

      // These round-trips exercise only the business-profile admin API (no
      // connector calls), so they are pinned to a single connector pipeline
      // to avoid redundant cross-connector execution. Skip if the connector
      // is NOT in the gating list.
      const shouldSkip =
        Array.isArray(gatingConnectors) &&
        !gatingConnectors.includes(connectorId);

      if (shouldSkip) {
        this.skip();
      }
    });

    it("Create Business Profile with full webhook event status lists", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["WebhookConfig"]["Create"];
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        webhook_details: data.Request.webhook_details,
      };
      cy.createBusinessProfileTest(createBody, globalState, "webhookGating");
    });

    // The shrink round-trips below assert set-equality on the echoed arrays
    // (expectWebhookStatusMembers), so a server-side merge-instead-of-replace
    // update would fail by retaining removed statuses. This replacement
    // semantics path is not covered by the Create/Update tests above, which
    // only ever grow the lists.
    it("Update payment_statuses_enabled to failed only (removes succeeded)", () => {
      const updateBody = {
        webhook_details: {
          payment_statuses_enabled: ["failed"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    it("Re-enable payment succeeded alongside failed", () => {
      const updateBody = {
        webhook_details: {
          payment_statuses_enabled: ["succeeded", "failed"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    it("Update refund_statuses_enabled to failure only (removes success)", () => {
      const updateBody = {
        webhook_details: {
          refund_statuses_enabled: ["failure"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    it("Update dispute_statuses_enabled to dispute_opened only (removes dispute_won)", () => {
      const updateBody = {
        webhook_details: {
          dispute_statuses_enabled: ["dispute_opened"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    it("Expand mandate_statuses_enabled to active and revoked", () => {
      const updateBody = {
        webhook_details: {
          mandate_statuses_enabled: ["active", "revoked"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    it("Update mandate_statuses_enabled back to active only (removes revoked)", () => {
      const updateBody = {
        webhook_details: {
          mandate_statuses_enabled: ["active"],
        },
      };
      cy.updateBusinessProfileWebhookConfigTest(
        updateBody,
        globalState,
        "webhookGating"
      );
    });

    // invoice_statuses_enabled is intentionally not round-tripped: outgoing
    // invoice webhooks have a single emission status (invoice_paid), so no
    // shrink/exclusion shape exists. Its echo is already asserted by the
    // WebhookConfig Create/Update tests above.

    after("cleanup webhookGating profile", () => {
      cy.deleteBusinessProfileTest(globalState, "webhookGating");
    });
  });

  context("Webhook Config Disabled Events — Negative Cases", () => {
    beforeEach(function () {
      const connectorId = globalState.get("connectorId");
      const webhookConfigConnectors =
        utils.CONNECTOR_LISTS.INCLUDE.WEBHOOK_CONFIG;

      // Skip if connector is NOT in the webhook config list
      const shouldSkip =
        Array.isArray(webhookConfigConnectors) &&
        !webhookConfigConnectors.includes(connectorId);

      if (shouldSkip) {
        this.skip();
      }
    });

    it("Create Business Profile with invalid refund_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "negative_webhook_test",
        webhook_details: {
          webhook_version: "1.0.2",
          payment_statuses_enabled: ["succeeded"],
          refund_statuses_enabled: ["succeeded"],
          payout_statuses_enabled: ["success"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegativeProfile",
        400
      );
    });

    it("Create Business Profile with invalid payment_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "neg_payment_status_test",
        webhook_details: {
          webhook_version: "1.0.2",
          payment_statuses_enabled: ["invalid_status"],
          refund_statuses_enabled: ["success", "failure"],
          payout_statuses_enabled: ["success", "failed"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegPaymentProfile",
        400
      );
    });

    it("Create Business Profile with invalid payout_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "neg_payout_status_test",
        webhook_details: {
          webhook_version: "1.0.2",
          payment_statuses_enabled: ["succeeded"],
          refund_statuses_enabled: ["success", "failure"],
          payout_statuses_enabled: ["invalid_payout"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegPayoutProfile",
        400
      );
    });

    it("Create Business Profile with invalid dispute_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "neg_dispute_status_test",
        webhook_details: {
          webhook_version: "1.0.2",
          dispute_statuses_enabled: ["invalid_dispute"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegDisputeProfile",
        400
      );
    });

    it("Create Business Profile with unsupported mandate_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "neg_mandate_status_test",
        webhook_details: {
          webhook_version: "1.0.2",
          mandate_statuses_enabled: ["inactive"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegMandateProfile",
        422
      );
    });

    it("Create Business Profile with unsupported invoice_statuses_enabled — expect error", () => {
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: "neg_invoice_status_test",
        webhook_details: {
          webhook_version: "1.0.2",
          invoice_statuses_enabled: ["payment_failed"],
        },
      };
      cy.createBusinessProfileTest(
        createBody,
        globalState,
        "webhookNegInvoiceProfile",
        422
      );
    });

    after("cleanup negative-case profiles", () => {
      cy.deleteBusinessProfileTest(globalState, "webhookNegativeProfile");
      cy.deleteBusinessProfileTest(globalState, "webhookNegPaymentProfile");
      cy.deleteBusinessProfileTest(globalState, "webhookNegPayoutProfile");
      cy.deleteBusinessProfileTest(globalState, "webhookNegDisputeProfile");
      cy.deleteBusinessProfileTest(globalState, "webhookNegMandateProfile");
      cy.deleteBusinessProfileTest(globalState, "webhookNegInvoiceProfile");
    });
  });
});
