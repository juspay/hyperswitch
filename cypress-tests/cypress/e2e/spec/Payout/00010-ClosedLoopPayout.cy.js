import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";
import getPaymentConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("[Payout] [Bank Transfer - Open Banking]", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_TRANSFER_OPEN_BANKING_PASSTHROUGH.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  // Closed-loop payout (PR #14535): the passthrough tokens used below are
  // taken from a live truelayer payment - sender_payment_instrument_id and
  // connector_customer_id of the payment response become the payout's
  // psp_token and psp_customer_id - and are sent with token_type
  // `open_banking` on the bank payout rail, which PR #14535 maps to the UCS
  // OPEN_BANKING payout method while forwarding billing_descriptor.reference
  // and statement_descriptor to UCS.
  context(
    "[Payout] [Bank transfer - Open Banking] Closed-loop passthrough (UCS)",
    // These tests form a stateful chain (confirm payment -> redirect ->
    // retrieve source tokens -> payout) held only in the describe-level
    // globalState. A retry resets the page and re-seeds globalState from
    // the node task, wiping every value the earlier tests produced - the
    // redirect retry then "passes" vacuously with no nextActionUrl and the
    // real failure never surfaces. Retries must stay off for this chain.
    { retries: 0 },
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach(function () {
        if (this.currentTest.state === "failed") {
          shouldContinue = false;
        }

        // Persist state to the node task after every test. In headless run
        // mode the first cross-origin navigation of the redirect journey
        // (integ -> truelayer-sandbox form auto-submit) makes Cypress
        // reload the spec on the new origin and re-execute the current
        // test with globalState re-seeded from this snapshot - without it,
        // paymentID/nextActionUrl produced by the payment leg are lost.
        cy.task("setGlobalState", globalState.data);
      });

      // sender_payment_instrument_id / connector_customer_id are returned
      // only by UCS-routed truelayer payments, so UCS must be enabled
      // before the closed-loop payment is created (same pattern as the
      // trustly open banking setup in 00004-BankTransfer.cy.js).
      it("create-ucs-configs-call-test", function () {
        if (!globalState.get("ucsEnabled")) {
          cy.task(
            "cli_log",
            "create-ucs-configs-call-test: UCS_ENABLED env not set - auto-enabling UCS for truelayer (UCS-only connector)"
          );
          globalState.set("ucsEnabled", true);
        }

        cy.setupConfigs(globalState, "ucs_enabled", "true");

        const proxyHttp = globalState.get("proxyHttp");
        const proxyHttps = globalState.get("proxyHttps");

        // Authorize covers the confirm call; PSync covers the force-sync
        // retrieve - without its rollout key the router falls back to the
        // legacy interface, which truelayer (UCS-only) does not implement.
        cy.createRolloutConfig(
          globalState,
          "bank_redirect_open_banking_Authorize,bank_redirect_open_banking_PSync",
          {
            rollout_percent: 1.0,
            execution_mode: "primary",
            ...(proxyHttp && proxyHttps
              ? { http_url: proxyHttp, https_url: proxyHttps }
              : {}),
          }
        );

        cy.createRolloutConfig(globalState, "Webhooks", {
          rollout_percent: 1.0,
          execution_mode: "primary",
          webhook_flows: ["Payout"],
        });
      });

      // The payout specs onboard only a payout_processor MCA (00002); the
      // closed-loop source payment additionally needs a payment_processor
      // MCA for bank_redirect/open_banking, otherwise payment routing finds
      // no eligible connector (IR_39). This mirrors the merchant setup used
      // in hyperswitch-cloud#23972. A distinct mcaPrefix keeps globalState
      // "merchantConnectorId" pointing at the payout MCA.
      it("create-truelayer-payment-connector-call-test", () => {
        const paymentConnectorBody = Cypress._.cloneDeep(
          fixtures.createConnectorBody
        );
        // The payout connector create (00002) shares this fixture and flips
        // test_mode to false for truelayer; the payment MCA mirrors the
        // payment-suite setup instead.
        paymentConnectorBody.test_mode = true;
        // Explicit label: the payout MCA onboarded in 00002 already owns
        // the auto-derived default label (truelayer_US_default) on this
        // profile, and (profile_id, connector_label) must be unique.
        paymentConnectorBody.connector_label = "truelayer_payment";

        cy.createPaymentConnectorIfAbsentCallTest(
          paymentConnectorBody,
          [
            {
              payment_method: "bank_redirect",
              payment_method_types: [
                {
                  payment_method_type: "open_banking",
                  payment_experience: null,
                  card_networks: null,
                  accepted_currencies: null,
                  accepted_countries: null,
                  minimum_amount: 1,
                  maximum_amount: 68607706,
                  recurring_enabled: true,
                  installment_payment_enabled: true,
                },
              ],
            },
          ],
          globalState,
          "profile",
          "paymentProcessorMerchantConnector"
        );
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      // Source payment of the closed loop. setup_future_usage=off_session
      // mirrors the merchant flow documented against PR #14535: the same
      // customer's stored bank authorisation is later referenced by the
      // payout through the passthrough tokens.
      it("create-and-confirm-truelayer-payment-test", () => {
        const intentData = {
          Request: {
            currency: "GBP",
            setup_future_usage: "off_session",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        };
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          intentData,
          "three_ds",
          "automatic",
          globalState
        );

        const confirmData = getPaymentConnectorDetails(
          globalState.get("connectorId")
        )["bank_redirect_pm"]["Truelayer"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      it("handle-bank-redirect-redirection-test", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      it("retrieve-payment-and-save-payout-source-details-test", () => {
        cy.retrieveClosedLoopPayoutSourceCallTest(globalState);
      });

      // cloneDeep both the shared payout fixture and the connector config:
      // billing_descriptor must not leak into the no-billing-descriptor
      // case below (or other specs), and the live passthrough tokens must
      // not pollute the shared config object.
      it("confirm-payout-call-with-auto-fulfill-test", () => {
        const pspToken = globalState.get("senderPaymentInstrumentId");
        const pspCustomerId = globalState.get("connectorCustomerId");
        // Fail fast with a clear message when the payment leg did not
        // produce the closed-loop source details, instead of sending a
        // payout create with missing passthrough tokens.
        expect(
          pspToken,
          "senderPaymentInstrumentId must be set by retrieve-payment-and-save-payout-source-details-test"
        ).to.be.a("string").and.to.not.be.empty;
        expect(
          pspCustomerId,
          "connectorCustomerId must be set by retrieve-payment-and-save-payout-source-details-test"
        ).to.be.a("string").and.to.not.be.empty;

        const data = Cypress._.cloneDeep(
          utils.getConnectorDetails(globalState.get("connectorId"))[
            "bank_transfer_pm"
          ]["open_banking"]["PassthroughBankPayout"]
        );
        data.Request.payout_method_data.passthrough.psp_token = pspToken;
        data.Request.payout_method_data.passthrough.psp_customer_id =
          pspCustomerId;

        cy.createConfirmPayoutTest(
          Cypress._.cloneDeep(fixtures.createPayoutBody),
          data,
          true,
          true,
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });

      // truelayer (via UCS) rejects payouts without
      // billing_descriptor.reference - the exact error PR #14535 surfaced
      // in hyperswitch-cloud#23972.
      it("confirm-payout-without-billing-descriptor-test", () => {
        const pspToken = globalState.get("senderPaymentInstrumentId");
        const pspCustomerId = globalState.get("connectorCustomerId");
        expect(
          pspToken,
          "senderPaymentInstrumentId must be set by retrieve-payment-and-save-payout-source-details-test"
        ).to.be.a("string").and.to.not.be.empty;
        expect(
          pspCustomerId,
          "connectorCustomerId must be set by retrieve-payment-and-save-payout-source-details-test"
        ).to.be.a("string").and.to.not.be.empty;

        const data = Cypress._.cloneDeep(
          utils.getConnectorDetails(globalState.get("connectorId"))[
            "bank_transfer_pm"
          ]["open_banking"]["PassthroughBankPayoutWithoutBillingDescriptor"]
        );
        data.Request.payout_method_data.passthrough.psp_token = pspToken;
        data.Request.payout_method_data.passthrough.psp_customer_id =
          pspCustomerId;

        cy.createConfirmPayoutTest(
          Cypress._.cloneDeep(fixtures.createPayoutBody),
          data,
          true,
          true,
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );
});
