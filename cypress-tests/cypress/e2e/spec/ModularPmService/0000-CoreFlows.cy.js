import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

/*
Modular Payment Method Service.

The modular payment method service is a separate deployment addressed by
PM_SERVICE_URL (see utils/State.js). Every flow in this spec goes through it,
so the whole spec is skipped when it is not provisioned — unlike
54-ConnectorAgnosticMandates, which falls back to the v1 equivalents.

The router only forwards payment method requests to the service once the org
is marked modular (system.should_call_pm_modular_service, org scoped), and
the service only hands out raw payment method details to external api keys
when payments.should_return_raw_payment_method_details allows it (merchant
scoped). Both are superposition configs (see dimension_config.rs for the
dimensions), set below once the merchant create has published the ids they
are scoped by — the KV setConfigs calls this spec used before are ignored
whenever superposition is deployed, as config resolution checks superposition
first and only falls back to the DB when that fetch fails.

Flow:
- Setup: merchant, api key and connector account, then the superposition
  configs above
- Customer and payment method core flows: customer create -> payment method
  create -> raw payment method details -> payment with the payment method id
  -> update the payment method -> list the saved payment methods
- Payment method session core flows: session create -> retrieve -> list ->
  update -> confirm -> get the payment method from the session token ->
  payment with the session token

Both payments are create + confirm pairs. The modular service returns saved
cards as card-with-optional-CVC, and only the confirm step collects the CVC
(payment_method_data.card_token) — the fetch the create step runs never
passes it. Connectors that do not implement card-with-optional-CVC (stripe,
unlike tsys) fail the single-call create+confirm with IR_00.
*/

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

        // The payment is created in two steps. The modular service hands the
        // saved card back as card-with-optional-CVC, and the create-side
        // fetch never passes the CVC along — a single create+confirm call
        // would reach stripe without one and fail with IR_00 (stripe, unlike
        // tsys, does not implement card-with-optional-CVC). Collecting the
        // CVC on a separate confirm (through payment_method_data.card_token,
        // the replacement for the deprecated top-level card_cvc) is the
        // designed path: the confirm-side fetch turns the card into a full
        // card before the connector call.

        // useToken false — the modular service's payment method ids are
        // valid payment tokens, so `payment_token` takes the pm_id
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

        // The status is only checked when it is passed explicitly,
        // `retrievePaymentCallTest` does not read `data.Response`
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

        // Same two-step shape as the pm_id payment above: the session token
        // resolves to the card the session confirm collected, and its CVC is
        // passed on the confirm through payment_method_data.card_token
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
