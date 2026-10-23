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

  context(
    "[Payout] [Bank transfer - Open Banking] Closed-loop passthrough (UCS)",
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

        cy.task("setGlobalState", globalState.data);
      });

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

      it("create-truelayer-payment-connector-call-test", () => {
        const paymentConnectorBody = Cypress._.cloneDeep(
          fixtures.createConnectorBody
        );
        paymentConnectorBody.test_mode = true;
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

      it("confirm-payout-call-with-auto-fulfill-test", () => {
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
