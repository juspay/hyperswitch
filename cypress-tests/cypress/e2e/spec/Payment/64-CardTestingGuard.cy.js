import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card Testing Guard", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.CARD_TESTING_GUARD
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card IP Blocking", () => {
    it("should block payment after card_ip_blocking threshold is reached", () => {
      let shouldContinue = true;

      cy.step("Enable only card_ip_blocking on business profile", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "enabled",
            card_ip_blocking_threshold: 3,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 100,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 100,
            card_testing_guard_expiry: 3600,
          },
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

      cy.step("Create Payment Intent - IP failure 1", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - IP failure 1"
          );
          return;
        }
        globalState.set("customerId", `ctg_ip_1_${Date.now()}`);
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment - IP failure 1", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment - IP failure 1");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - IP failure 2", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - IP failure 2"
          );
          return;
        }
        globalState.set("customerId", `ctg_ip_2_${Date.now()}`);
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
      });

      cy.step("Confirm Payment - IP failure 2", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment - IP failure 2");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - IP failure 3", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - IP failure 3"
          );
          return;
        }
        globalState.set("customerId", `ctg_ip_3_${Date.now()}`);
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
      });

      cy.step("Confirm Payment - IP failure 3", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment - IP failure 3");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - IP blocked attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - IP blocked attempt"
          );
          return;
        }
        globalState.set("customerId", `ctg_ip_4_${Date.now()}`);
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
      });

      cy.step("Confirm Payment - should be blocked by IP", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - should be blocked by IP"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["BlockedConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.step("Disable card_testing_guard_config", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 3,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 100,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 100,
            card_testing_guard_expiry: 3600,
          },
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
    });
  });

  context("Customer ID Blocking", () => {
    it("should block payment after customer_id_blocking threshold is reached", () => {
      let shouldContinue = true;

      cy.step("Enable only customer_id_blocking on business profile", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 100,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 100,
            customer_id_blocking_status: "enabled",
            customer_id_blocking_threshold: 2,
            card_testing_guard_expiry: 3600,
          },
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

      cy.step("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Intent - Customer ID failure 1", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Customer ID failure 1"
          );
          return;
        }
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment - Customer ID failure 1", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - Customer ID failure 1"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - Customer ID failure 2", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Customer ID failure 2"
          );
          return;
        }
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
      });

      cy.step("Confirm Payment - Customer ID failure 2", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - Customer ID failure 2"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - Customer ID blocked attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Customer ID blocked attempt"
          );
          return;
        }
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
      });

      cy.step("Confirm Payment - should be blocked by customer_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - should be blocked by customer_id"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["BlockedConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.step("Disable card_testing_guard_config", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 100,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 100,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 2,
            card_testing_guard_expiry: 3600,
          },
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
    });
  });

  context("Guest User Card Blocking", () => {
    it("should block payment after guest_user_card_blocking threshold is reached", () => {
      let shouldContinue = true;

      cy.step(
        "Enable only guest_user_card_blocking on business profile",
        () => {
          const updateBusinessProfileBody = {
            card_testing_guard_config: {
              card_ip_blocking_status: "disabled",
              card_ip_blocking_threshold: 100,
              guest_user_card_blocking_status: "enabled",
              guest_user_card_blocking_threshold: 2,
              customer_id_blocking_status: "disabled",
              customer_id_blocking_threshold: 100,
              card_testing_guard_expiry: 3600,
            },
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
        }
      );

      cy.step("Create Payment Intent - Guest failure 1", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Guest failure 1"
          );
          return;
        }
        globalState.set("customerId", undefined);
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment - Guest failure 1", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - Guest failure 1"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["GuestFailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - Guest failure 2", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Guest failure 2"
          );
          return;
        }
        globalState.set("customerId", undefined);
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
      });

      cy.step("Confirm Payment - Guest failure 2", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - Guest failure 2"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["GuestFailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - Guest blocked attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - Guest blocked attempt"
          );
          return;
        }
        globalState.set("customerId", undefined);
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
      });

      cy.step("Confirm Payment - should be blocked by guest user rule", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - should be blocked by guest user rule"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["GuestBlockedConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.step("Disable card_testing_guard_config", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 100,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 2,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 100,
            card_testing_guard_expiry: 3600,
          },
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
    });
  });

  context("Successful Payment Below Threshold", () => {
    it("should allow payment when failures are below threshold", () => {
      let shouldContinue = true;

      cy.step(
        "Enable customer_id_blocking with threshold 2 on business profile",
        () => {
          const updateBusinessProfileBody = {
            card_testing_guard_config: {
              card_ip_blocking_status: "disabled",
              card_ip_blocking_threshold: 100,
              guest_user_card_blocking_status: "disabled",
              guest_user_card_blocking_threshold: 100,
              customer_id_blocking_status: "enabled",
              customer_id_blocking_threshold: 2,
              card_testing_guard_expiry: 3600,
            },
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
        }
      );

      cy.step("Create Customer for below-threshold test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Intent - single failure below threshold", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - single failure below threshold"
          );
          return;
        }
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment - single failure (below threshold)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - single failure (below threshold)"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardTestingGuard"]["FailConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent - success attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent - success attempt"
          );
          return;
        }
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
      });

      cy.step("Confirm Payment - should succeed below threshold", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - should succeed below threshold"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.step("Disable card_testing_guard_config", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 100,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 100,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 2,
            card_testing_guard_expiry: 3600,
          },
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
    });
  });

  context("Disable Card Testing Guard Config", () => {
    it("should not block payment when card_testing_guard is disabled", () => {
      let shouldContinue = true;

      cy.step("Disable all card_testing_guard mechanisms", () => {
        const updateBusinessProfileBody = {
          card_testing_guard_config: {
            card_ip_blocking_status: "disabled",
            card_ip_blocking_threshold: 3,
            guest_user_card_blocking_status: "disabled",
            guest_user_card_blocking_threshold: 2,
            customer_id_blocking_status: "disabled",
            customer_id_blocking_threshold: 2,
            card_testing_guard_expiry: 3600,
          },
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

      cy.step(
        "Create Payment Intent - should succeed with guard disabled",
        () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Confirm Payment - should succeed with guard disabled", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment - should succeed with guard disabled"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });
  });
});
