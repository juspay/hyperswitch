import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Subscription Management tests", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.SUBSCRIPTION.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create Subscription", () => {
    it("create-subscription-happy-path-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Create"];

      cy.createSubscriptionTest(
        fixtures.createSubscriptionBody,
        data,
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("retrieve-created-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Subscription");
        return;
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

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
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

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });
  });

  context("Update Subscription", () => {
    it("update-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Update Subscription");
        return;
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

    it("verify-updated-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Verify Updated Subscription");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Cancel Subscription", () => {
    it("cancel-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Cancel Subscription");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Cancel"];

      cy.cancelSubscriptionTest(data, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-cancelled-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Verify Cancelled Subscription");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["RetrieveCancelled"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Reactivate Subscription", () => {
    it("reactivate-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Reactivate Subscription");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Reactivate"];

      cy.reactivateSubscriptionTest(data, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("verify-reactivated-subscription-test", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Verify Reactivated Subscription");
        return;
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "subscription_pm"
      ]["Retrieve"];

      cy.retrieveSubscriptionTest(data, globalState);
    });
  });

  context("Subscription Lifecycle Flow", () => {
    it("full-lifecycle-create-subscription-test", () => {
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
