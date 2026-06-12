import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { PaymentUpdateClientAuthConfigs } from "../../configs/Payment/ClientPaymentUpdate";

let connector;
let globalState;

describe("Payment Update via Client Authentication Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.PAYMENT_UPDATE_CLIENT_AUTH
          )
        ) {
          skip = true;
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

  context("Payment Update via Client Auth - Happy Path", () => {
    it("Enable config -> Create Payment Intent -> Update via Client Auth -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Enable Payment Update Client Auth Config", () => {
        const merchantId = globalState.get("merchantId");
        const configKey = `payment_update_enabled_for_client_auth_${merchantId}`;
        const baseUrl = globalState.get("baseUrl");
        const apiKey = globalState.get("adminApiKey");
        cy.request({
          method: "POST",
          url: `${baseUrl}/configs/`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            key: configKey,
            value: "true",
          },
          failOnStatusCode: false,
        });
      });

      cy.step("Create Payment Intent", () => {
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

      cy.step("Update Payment via Client Authentication", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Update Payment via Client Authentication"
          );
          return;
        }

        const data = PaymentUpdateClientAuthConfigs.HappyPath;

        cy.paymentUpdateClientAuthTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Configs: {
              skipBillingAssertion: true,
            },
          },
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Payment Update via Client Auth - Feature Disabled Error", () => {
    it("Failed update when feature is disabled", () => {
      let shouldContinue = true;

      cy.step("Ensure config is disabled", () => {
        const merchantId = globalState.get("merchantId");
        const configKey = `payment_update_enabled_for_client_auth_${merchantId}`;
        const baseUrl = globalState.get("baseUrl");
        const apiKey = globalState.get("adminApiKey");
        cy.request({
          method: "POST",
          url: `${baseUrl}/configs/${configKey}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            key: configKey,
            value: "false",
          },
          failOnStatusCode: false,
        });
      });

      cy.step("Create Payment Intent", () => {
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

      cy.step("Attempt update with feature disabled", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Attempt update with feature disabled"
          );
          return;
        }

        cy.paymentUpdateClientAuthTest(
          globalState,
          PaymentUpdateClientAuthConfigs.FeatureDisabled
        );
      });
    });
  });

  context(
    "Payment Update via Client Auth - Invalid Client Secret Error",
    () => {
      it("Failed update with invalid client secret", () => {
        let shouldContinue = true;

        cy.step("Enable config", () => {
          const merchantId = globalState.get("merchantId");
          const configKey = `payment_update_enabled_for_client_auth_${merchantId}`;
          const baseUrl = globalState.get("baseUrl");
          const apiKey = globalState.get("adminApiKey");
          cy.request({
            method: "POST",
            url: `${baseUrl}/configs/`,
            headers: {
              "Content-Type": "application/json",
              "api-key": apiKey,
            },
            body: {
              key: configKey,
              value: "true",
            },
            failOnStatusCode: false,
          });
        });

        cy.step("Create Payment Intent", () => {
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

        cy.step("Attempt update with invalid client secret", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Attempt update with invalid client secret"
            );
            return;
          }

          cy.paymentUpdateClientAuthTest(
            globalState,
            PaymentUpdateClientAuthConfigs.InvalidClientSecret
          );
        });
      });
    }
  );

  context("Payment Update via Client Auth - Wrong Customer ID Error", () => {
    it("Failed update with wrong customer ID", () => {
      let shouldContinue = true;

      cy.step("Enable config", () => {
        const merchantId = globalState.get("merchantId");
        const configKey = `payment_update_enabled_for_client_auth_${merchantId}`;
        const baseUrl = globalState.get("baseUrl");
        const apiKey = globalState.get("adminApiKey");
        cy.request({
          method: "POST",
          url: `${baseUrl}/configs/`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            key: configKey,
            value: "true",
          },
          failOnStatusCode: false,
        });
      });

      cy.step("Create Payment Intent", () => {
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

      cy.step("Attempt update with wrong customer ID", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Attempt update with wrong customer ID"
          );
          return;
        }

        cy.paymentUpdateClientAuthTest(
          globalState,
          PaymentUpdateClientAuthConfigs.WrongCustomerId
        );
      });
    });
  });

  context("Payment Update via Client Auth - Wrong Payment Status Error", () => {
    it("Failed update when payment status does not allow updates", () => {
      let shouldContinue = true;

      cy.step("Enable config", () => {
        const merchantId = globalState.get("merchantId");
        const configKey = `payment_update_enabled_for_client_auth_${merchantId}`;
        const baseUrl = globalState.get("baseUrl");
        const apiKey = globalState.get("adminApiKey");
        cy.request({
          method: "POST",
          url: `${baseUrl}/configs/`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: {
            key: configKey,
            value: "true",
          },
          failOnStatusCode: false,
        });
      });

      cy.step("Create and Confirm Payment to reach succeeded status", () => {
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Attempt update on already-confirmed payment", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Attempt update on already-confirmed payment"
          );
          return;
        }

        cy.paymentUpdateClientAuthTest(
          globalState,
          PaymentUpdateClientAuthConfigs.WrongPaymentStatus
        );
      });
    });
  });
});
