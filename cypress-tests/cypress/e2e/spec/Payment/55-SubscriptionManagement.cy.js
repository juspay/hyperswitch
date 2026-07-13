// Spec #55. Number 56 is reserved by PR #13154 (DynamicFields was moved to 57 to avoid conflict).
import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Subscription Management tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (!Cypress.env("STRIPE_TEST_PRICE_ID")) {
        cy.task(
          "cli_log",
          "STRIPE_TEST_PRICE_ID is required for subscription create/update tests. Configure a valid Stripe test price ID before running this spec."
        );
      }
    });
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
    it("create-payment-method-for-subscription-test", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
      cy.createPaymentMethodTest(globalState, data);
    });

    it("create-billing-connector-test", () => {
      cy.createBillingConnectorTest(
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState,
        globalState.get("connectorId"),
        "stripebilling"
      );
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

  context("Create Subscription", () => {
    it("create-subscription-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Create"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionBody,
        data,
        globalState
      );
    });

    it("retrieve-created-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Create Subscription - Negative Cases", () => {
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
    it("update-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Update"];

      cy.updateSubscriptionTest(
        fixtures.updateSubscriptionBody,
        data,
        globalState
      );
    });

    it("verify-updated-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Cancel Subscription", () => {
    it("cancel-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Cancel"];

      cy.cancelSubscriptionTest(data, globalState);
    });

    it("verify-cancelled-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["RetrieveCancelled"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Resume Subscription", () => {
    it("resume-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Resume"];

      cy.resumeSubscriptionTest(data, globalState);
    });

    it("verify-resumed-subscription-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });
});
