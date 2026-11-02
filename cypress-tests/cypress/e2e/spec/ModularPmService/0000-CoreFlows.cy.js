import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Modular Payment Method Service - Core Flows", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // The whole spec depends on the modular payment method service
      if (!globalState.get("pmServiceUrl")) {
        cy.task("cli_log", "PM_SERVICE_URL not set — skipping the spec");
        specShouldSkip = true;
        return;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup - merchant, api key and connector account", () => {
    it("merchant create call", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("API key create call", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("Connector create call", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });
  });

  // Both configs stay in place after the run — the merchant (and with it the
  // org and merchant ids they are scoped by) is created fresh on every run
  context("Setup - superposition configs", () => {
    it("org - enable the modular payment method service", () => {
      cy.setSuperpositionConfigs(
        globalState,
        { "system.should_call_pm_modular_service": true },
        // DimensionsWithProviderMerchantIdAndOrgId; the org scoped context
        // is what 54-ConnectorAgnosticMandates uses for the same config
        { organization_id: globalState.get("organizationId") }
      );
    });

    it("merchant - enable raw payment method details", () => {
      cy.setSuperpositionConfigs(
        globalState,
        { "payments.should_return_raw_payment_method_details": true },
        // DimensionsWithProviderMerchantId — this config only resolves
        // against a merchant scoped context
        { provider_merchant_id: globalState.get("merchantId") }
      );
    });
  });

  context("Customer and payment method core flows", () => {
    it("Customer create call", () => {
      cy.customerCreateCall(globalState, fixtures.customerCreate);
    });

    it("Payment Method Create call", () => {
      cy.paymentMethodCreateCall(globalState, fixtures.paymentMethodCreate);
    });

    it("Get Raw Payment Method Details call", () => {
      cy.getRawPaymentMethodDetailsCall(
        globalState,
        fixtures.paymentMethodCreate
      );
    });

    it("Payments call with pm_id -> Retrieve Payment", () => {
      cy.step("Payments call with pm_id", () => {
        // `retrievePaymentCallTest` asserts the retrieved amount against
        // `paymentAmount`, which `paymentWithSavedPMCall` does not set
        globalState.set(
          "paymentAmount",
          fixtures.modularPmServicePaymentsCall.amount
        );

        cy.paymentWithSavedPMCall(globalState, {
          ...fixtures.modularPmServicePaymentsCall,
          confirm: false,
        });

        cy.confirmWithSavedPMCall(globalState, {
          ...fixtures.modularPmServicePaymentsCall,
          payment_method_data: {
            card_token: {
              card_cvc:
                fixtures.paymentMethodCreate.payment_method_data.card.card_cvc,
            },
          },
        });
      });

      cy.step("Retrieve Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "succeeded",
        });
      });
    });

    it("Update Payment Method call", () => {
      cy.updateSavedPMCall(globalState, fixtures.paymentMethodUpdate);
    });

    it("Payment Method List call", () => {
      cy.listSavedPMCall(globalState);
    });
  });

  context("Payment method session core flows", () => {
    it("Payment Method Session Create call", () => {
      cy.pmSessionCreateCall(globalState, fixtures.paymentMethodSessionCreate);
    });

    it("Payment Method Session Retrieve call", () => {
      cy.pmSessionRetrieveCall(globalState);
    });

    it("Payment Method Session List call", () => {
      cy.pmSessionListPMCall(globalState);
    });

    it("Payment Method Session Update call", () => {
      cy.pmSessionUpdatePMCall(
        globalState,
        fixtures.paymentMethodSessionUpdate
      );
    });

    it("Payment Method Session Confirm call", () => {
      cy.pmSessionConfirmCall(
        globalState,
        fixtures.paymentMethodSessionConfirm
      );
    });

    it("Get Payment Method from session token call", () => {
      cy.getPMFromTokenCall(globalState);
    });

    it("Payments call with pm_token -> Retrieve Payment", () => {
      cy.step("Payments call with pm_token", () => {
        globalState.set(
          "paymentAmount",
          fixtures.modularPmServicePaymentsCall.amount
        );

        cy.paymentWithSavedPMCall(
          globalState,
          {
            ...fixtures.modularPmServicePaymentsCall,
            confirm: false,
          },
          true
        );

        cy.confirmWithSavedPMCall(
          globalState,
          {
            ...fixtures.modularPmServicePaymentsCall,
            payment_method_data: {
              card_token: {
                card_cvc:
                  fixtures.paymentMethodSessionConfirm.payment_method_data.card
                    .card_cvc,
              },
            },
          },
          true
        );
      });

      cy.step("Retrieve Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "succeeded",
        });
      });
    });
  });
});
