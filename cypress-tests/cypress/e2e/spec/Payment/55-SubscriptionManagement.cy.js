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

  context("Prerequisites", () => {
    const prereqContinue = true;

    beforeEach(function () {
      if (!prereqContinue) {
        this.skip();
      }
    });

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
    it("create-subscription-happy-path-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Create"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        this.skip();
      }

      cy.createSubscriptionTest(
        fixtures.createSubscriptionBody,
        data,
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
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

  context("Reactivate Subscription", () => {
    it("reactivate-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Reactivate"];

      cy.reactivateSubscriptionTest(data, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-reactivated-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Subscription Lifecycle Flow", () => {
    it("full-lifecycle-create-subscription-test", function () {
      if (!shouldContinue) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Create"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionBody,
        data,
        globalState
      );

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }

      if (!shouldContinue) {
        cy.task("cli_log", "Skipping remaining lifecycle steps");
        return;
      }

      const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(retrieveData, globalState);

      const updateData = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Update"];

      cy.updateSubscriptionTest(
        fixtures.updateSubscriptionBody,
        updateData,
        globalState
      );

      if (!utils.should_continue_further(updateData)) {
        shouldContinue = false;
      }

      if (!shouldContinue) {
        cy.task("cli_log", "Skipping cancel step");
        return;
      }

      const cancelData = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Cancel"];

      cy.cancelSubscriptionTest(cancelData, globalState);

      if (!utils.should_continue_further(cancelData)) {
        shouldContinue = false;
      }

      if (!shouldContinue) {
        cy.task("cli_log", "Skipping final retrieve step");
        return;
      }

      const retrieveCancelledData = getConnectorDetails(
        globalState.get("connectorId")
      )["subscription_pm"]["RetrieveCancelled"];

      cy.retrieveSubscriptionTest(retrieveCancelledData, globalState);
    });
  });
});
