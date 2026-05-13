import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

const bpUpdateRedirectPost = {
  is_connector_agnostic_mit_enabled: true,
  collect_shipping_details_from_wallet_connector: false,
  collect_billing_details_from_wallet_connector: false,
  always_collect_billing_details_from_wallet_connector: false,
  always_collect_shipping_details_from_wallet_connector: false,
  redirect_to_merchant_with_http_post: true,
};

const bpUpdateRedirectGet = {
  is_connector_agnostic_mit_enabled: true,
  collect_shipping_details_from_wallet_connector: false,
  collect_billing_details_from_wallet_connector: false,
  always_collect_billing_details_from_wallet_connector: false,
  always_collect_shipping_details_from_wallet_connector: false,
  redirect_to_merchant_with_http_post: false,
};

describe("Merchant Redirect Method Tests - UPI", () => {
  before(
    "seed global state and create business profile with connector",
    function () {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);

        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );

        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });
    }
  );

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Test redirect_to_merchant_with_http_post enabled - POST redirect flow",
    () => {
      it("Enable POST redirect → Create UPI Payment → Confirm → Handle Redirect → Verify Return URL", () => {
        let shouldContinue = true;

        cy.step("Update redirect_to_merchant_with_http_post to true", () => {
          cy.UpdateBusinessProfileTest(
            bpUpdateRedirectPost,
            true,
            false,
            false,
            false,
            false,
            globalState
          );
        });

        cy.step("Create Payment Intent with UPI", () => {
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["upi_pm"]?.["PaymentIntent"];
          expect(data, `upi_pm.PaymentIntent not found for ${connectorId}`).to
            .exist;

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm UPI payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["upi_pm"]?.["UpiCollect"];
          expect(data, `upi_pm.UpiCollect not found for ${connectorId}`).to
            .exist;

          if (!data) {
            cy.task(
              "cli_log",
              "Skipping confirm step: UpiCollect config not found"
            );
            shouldContinue = false;
            return;
          }

          cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);
          if (!should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step(
          "Handle UPI redirect and verify return URL parameters (POST mode)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Handle Redirect");
              return;
            }
            const expected_redirection = fixtures.confirmBody["return_url"];
            const payment_method_type = globalState.get("paymentMethodType");

            cy.handleUpiRedirection(
              globalState,
              payment_method_type,
              expected_redirection
            );

            cy.verifyUrlParamExcluded(
              "amount",
              "Verified: amount parameter is excluded (POST mode)"
            );
          }
        );
      });
    }
  );

  context(
    "Test redirect_to_merchant_with_http_post disabled - GET redirect flow",
    () => {
      it("Enable GET redirect → Create UPI Payment → Confirm → Handle Redirect → Verify Return URL", () => {
        let shouldContinue = true;

        cy.step("Update redirect_to_merchant_with_http_post to false", () => {
          cy.UpdateBusinessProfileTest(
            bpUpdateRedirectGet,
            true,
            false,
            false,
            false,
            false,
            globalState
          );
        });

        cy.step("Create Payment Intent with UPI", () => {
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["upi_pm"]?.["PaymentIntent"];
          expect(data, `upi_pm.PaymentIntent not found for ${connectorId}`).to
            .exist;

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm UPI payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["upi_pm"]?.["UpiCollect"];
          expect(data, `upi_pm.UpiCollect not found for ${connectorId}`).to
            .exist;

          if (!data) {
            cy.task(
              "cli_log",
              "Skipping confirm step: UpiCollect config not found"
            );
            shouldContinue = false;
            return;
          }

          cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);
          if (!should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step(
          "Handle UPI redirect and verify return URL parameters (GET mode)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Handle Redirect");
              return;
            }
            const expected_redirection = fixtures.confirmBody["return_url"];
            const payment_method_type = globalState.get("paymentMethodType");

            cy.handleUpiRedirection(
              globalState,
              payment_method_type,
              expected_redirection
            );

            cy.verifyUrlParamIncluded(
              "amount",
              "Verified: amount parameter is included (GET mode)"
            );
          }
        );
      });
    }
  );
});
