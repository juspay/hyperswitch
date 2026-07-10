// Spec #55. Number 56 is reserved by PR #13154 (DynamicFields was moved to 57 to avoid conflict).
import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Subscription Management tests", () => {
  let shouldContinue = true;
  let connectorSupported = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.SUBSCRIPTION.includes(
          globalState.get("connectorId")
        )
      ) {
        connectorSupported = false;
      }
      if (!Cypress.env("STRIPE_TEST_PRICE_ID")) {
        cy.task(
          "cli_log",
          "WARNING: STRIPE_TEST_PRICE_ID not set — subscription Create tests will be skipped. Set this env var with a valid Stripe test price ID to enable full coverage."
        );
      }
    });
  });

  beforeEach(function () {
    if (!connectorSupported) {
      this.skip();
    }
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("cleanup billing connector", () => {
    cy.deleteBillingConnectorTest(globalState).then(() => {
      cy.task("setGlobalState", globalState.data);
    });
  });

  context("Prerequisites", () => {
    let prereqContinue = true;

    beforeEach(function () {
      if (!shouldContinue || !prereqContinue) {
        this.skip();
      }
    });

    it("create-payment-method-for-subscription-test", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
      cy.createPaymentMethodTest(globalState, data);
      cy.wrap(null).then(() => {
        shouldContinue = !!globalState.get("paymentMethodId");
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping billing connector setup: payment method creation failed");
        }
      });
    });

    it("create-billing-connector-test", () => {
      cy.createBillingConnectorTest(
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState,
        globalState.get("connectorId"),
        "stripebilling"
      );
      cy.wrap(null).then(() => {
        if (shouldContinue) {
          shouldContinue = !!globalState.get("billingProcessorConnectorId");
        }
      });
    });

    it("configure-billing-processor-id-test", () => {
      const bpBody = {
        ...fixtures.businessProfile.bpUpdate,
        billing_processor_id: globalState.get("billingProcessorConnectorId"),
      };
      cy.UpdateBusinessProfileTest(
        bpBody,
        true, // is_connector_agnostic_mit_enabled
        false, // collect_billing_details_from_wallet_connector
        false, // collect_shipping_details_from_wallet_connector
        false, // always_collect_billing_details_from_wallet_connector
        false, // always_collect_shipping_details_from_wallet_connector
        globalState
      );
    });
  });

  context("Create Subscription - Known Limitation", () => {
    it("create-subscription-known-limitation-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Create"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionBody,
        data,
        globalState
      );

      shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-created-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Create Subscription - Negative Cases", () => {
    beforeEach(function () {
      if (!globalState.get("billingProcessorConnectorId")) {
        this.skip();
      }
    });

    it("create-subscription-invalid-customer-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["CreateInvalidCustomer"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionInvalidCustomerBody,
        data,
        globalState
      );
    });

    it("create-subscription-missing-fields-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["CreateMissingFields"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionMissingFieldsBody,
        data,
        globalState
      );
    });
  });

  context("Update Subscription", () => {
    it("update-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Update"];

      cy.updateSubscriptionTest(
        fixtures.updateSubscriptionBody,
        data,
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-updated-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Cancel Subscription", () => {
    it("cancel-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Cancel"];

      cy.cancelSubscriptionTest(data, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-cancelled-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["RetrieveCancelled"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Resume Subscription", () => {
    it("resume-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Resume"];

      cy.resumeSubscriptionTest(data, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-resumed-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });
});
