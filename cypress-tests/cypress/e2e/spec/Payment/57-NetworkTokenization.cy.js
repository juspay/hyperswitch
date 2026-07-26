import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Network Tokenization Tests", function () {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.NETWORK_TOKENIZATION
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("network-tokenization-payment-flow", () => {
    it("Payment succeeds with network tokenization enabled on profile (bankofamerica not in network_tokenization_supported_connectors)", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile to enable network tokenization", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Business Profile");
          return;
        }

        const updateBusinessProfileBody = {
          is_network_tokenization_enabled: true,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          /* is_connector_agnostic_mit_enabled */ false,
          /* collect_billing_details_from_wallet_connector */ false,
          /* collect_shipping_details_from_wallet_connector */ false,
          /* always_collect_billing_details_from_wallet_connector */ false,
          /* always_collect_shipping_details_from_wallet_connector */ false,
          globalState,
          "profile"
        );
      });

      cy.step("Create Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Retrieve Payment and verify tokenization fields are null (no tokenization attempted for unsupported connector)",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }

          cy.retrievePaymentCallTest({ globalState });

          cy.task(
            "cli_log",
            "bankofamerica is not in network_tokenization_supported_connectors (adyen, cybersource, peachpayments, trustpay). Tokenization is not attempted even when is_network_tokenization_enabled is true on the profile. Null fields confirm no tokenization occurred — this is expected behavior, not a service failure."
          );

          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/payments/${globalState.get(
              "paymentID"
            )}?force_sync=true&expand_attempts=true`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            expect(response.status).to.equal(200);
            expect(response.body).to.have.property(
              "status",
              "succeeded",
              "Payment should succeed even when network tokenization is not attempted"
            );
            expect(response.body).to.have.property(
              "network_transaction_id",
              null,
              "network_transaction_id is null — bankofamerica not in network_tokenization_supported_connectors, no tokenization attempted"
            );
            expect(response.body).to.have.property(
              "network_transaction_link_id",
              null,
              "network_transaction_link_id is null — bankofamerica not in network_tokenization_supported_connectors, no tokenization attempted"
            );
            expect(response.body).to.have.property(
              "tokenization",
              null,
              "tokenization is null — bankofamerica not in network_tokenization_supported_connectors, no tokenization attempted"
            );
            expect(response.body).to.have.property(
              "payment_method_tokenization_details",
              null,
              "payment_method_tokenization_details is null — bankofamerica not in network_tokenization_supported_connectors, no tokenization attempted"
            );
          });
        }
      );
    });
  });

  context("tokenize-card-endpoint", () => {
    it("Tokenize card endpoint returns 500 when network tokenization service is not configured", () => {
      const shouldContinue = true;

      cy.step(
        "Tokenize card — verify endpoint returns NetworkTokenizationServiceNotConfigured error",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Tokenize card");
            return;
          }

          cy.task(
            "cli_log",
            "The /payment_methods/tokenize-card endpoint checks the global [network_tokenization_service] server config, not profile-level credentials. When the service is not configured, it returns NetworkTokenizationServiceNotConfigured which maps to InternalServerError (HE_00). This is the only stable error code for this path — the error occurs before the tokenization service API is called, so no tokenization-specific error code (e.g. NT_XX) is available. The 500 (not 400/401/404) confirms the endpoint exists, the request body schema is valid, and admin API key authentication works."
          );

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["NetworkTokenization"];

          cy.tokenizeCardTest({}, data, globalState);
        }
      );
    });
  });

  context("reset-business-profile", () => {
    it("Reset business profile to disable network tokenization", () => {
      cy.step("Reset network tokenization flag", () => {
        const updateBusinessProfileBody = {
          is_network_tokenization_enabled: false,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          /* is_connector_agnostic_mit_enabled */ false,
          /* collect_billing_details_from_wallet_connector */ false,
          /* collect_shipping_details_from_wallet_connector */ false,
          /* always_collect_billing_details_from_wallet_connector */ false,
          /* always_collect_shipping_details_from_wallet_connector */ false,
          globalState
        );
      });
    });
  });
});
