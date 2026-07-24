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
    it("Enable Network Tokenization -> Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile to enable network tokenization", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Business Profile");
          return;
        }

        const updateBusinessProfileBody = {
          is_network_tokenization_enabled: true,
          network_tokenization_credentials: {
            internal_network_token_service: {
              token_service_api_key: "test_token_service_key",
              public_key: "test_public_key",
              private_key: "test_private_key",
              key_id: "test_key_id",
            },
          },
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

      cy.step("Retrieve Payment and verify network tokenization fields", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("tokenize-card-endpoint", () => {
    it("Tokenize card via /payment_methods/tokenize-card (expect 500 with fake credentials)", () => {
      let shouldContinue = true;

      cy.step("Tokenize card", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Tokenize card");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["NetworkTokenization"];

        cy.tokenizeCardTest({}, data, globalState);
      });
    });
  });

  context("reset-business-profile", () => {
    it("Reset business profile to disable network tokenization", () => {
      cy.step("Reset network tokenization flags", () => {
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
