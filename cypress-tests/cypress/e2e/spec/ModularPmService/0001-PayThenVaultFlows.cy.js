import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

/*
Pay then vault (PtV) — juspay/hyperswitch PR #13832.

With the `system.payment_method_integration_type` superposition config set to
`pay_then_vault` (together with `system.should_call_pm_modular_service`), a
card confirmed in a payment method session is written to redis (volatile)
only, and is promoted to the card vault + payment method DB entry only after
the payment that uses it is acknowledged. The v2 payment method service
(PM_SERVICE_URL) serves the /v2/... paths used below; the v1 payments stay on
BASEURL.

Scenarios (mirroring the PR walkthrough):
- Customer + acceptance: persistent session -> confirm (volatile) -> payment
  -> the card is promoted and appears in the saved payment method list
- Duplicate card, same customer: a second payment with the same card keeps a
  single saved payment method (vault fingerprint dedupe)
- Customer without acceptance: payment succeeds but the card is never
  promoted — acceptance is the promotion gate
- Guest without acceptance: volatile session token pays successfully and the
  token details are retrievable
- Guest with acceptance: the payment succeeds but the guest payment method
  acknowledgement is rejected (404 HE_02) — guest cards are never promoted
*/

// Both PtV keys are merchant scoped (see dimension_config.rs), so they share
// one superposition context and are written in a single PUT.
const ptvSuperpositionOverrides = {
  "system.should_call_pm_modular_service": true,
  "system.payment_method_integration_type": "pay_then_vault",
};

const ptvSuperpositionContext = () => ({
  provider_merchant_id: globalState.get("merchantId"),
});

// BIN enrichment overrides the client sent subtype: 4111111111111111 is sent
// as `credit` but comes back (and is stored on the promoted payment method)
// as `debit` from the cards_info BIN table
const expectedEnrichedCard = {
  card_type: "DEBIT",
  funding_source: "DEBIT",
  card_network: "Visa",
  last4_digits: "1111",
};

// Expectations for a promoted card payment method, parametrised by the
// amount its connector token was last authorized for
const expectedPromotedPm = (authorizedAmount) => ({
  payment_method_type: "card",
  payment_method_subtype: "debit",
  recurring_enabled: true,
  requires_cvv: true,
  card_last4_digits: "1111",
  connector_token_type: "multi_use",
  connector_token_status: "active",
  connector_token_authorized_amount: authorizedAmount,
});

const ptvSaveCardData = () =>
  getConnectorDetails(globalState.get("connectorId"))["card_pm"][
    "PayThenVaultSaveCardOffSession"
  ];

// The v1 payment merges the connector config's Request over the fixture,
// mirroring `confirmWithSavedPaymentMethod` in 54-ConnectorAgnosticMandates
const ptvPaymentBody = (request, overrides = {}) => ({
  ...fixtures.modularPmServicePaymentsCall,
  ...request,
  ...overrides,
});

describe("Pay then vault flows", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Pay then vault setup", () => {
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

    it("enable pay then vault via superposition", () => {
      if (!globalState.get("pmServiceUrl")) {
        throw new Error(
          "PM_SERVICE_URL not set — the pay then vault flows need the v2 payment method service"
        );
      }
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret")
      ) {
        throw new Error(
          "SUPERPOSITION_BASE_URL / SUPERPOSITION_SECRET not set — pay then vault cannot be enabled"
        );
      }
      cy.setSuperpositionConfigs(
        globalState,
        ptvSuperpositionOverrides,
        ptvSuperpositionContext()
      );
      // The fixed 15s wait inside setSuperpositionConfigs only covers CI's
      // ~10s superposition polling; local dev routers poll every 300s, phase
      // shifted, and the confirm (PM service) and payment/promotion (v1
      // router) paths sit on different routers. Poll BOTH gateways until each
      // reflects the overrides before the first PtV-dependent assertion.
      cy.waitForPtVConfigPropagation(
        globalState,
        fixtures.customerCreate,
        fixtures.paymentMethodSessionCreate,
        fixtures.payThenVaultPmsConfirm
      );
    });
  });

  context(
    "Customer with acceptance - card is promoted after the payment",
    () => {
      it("create customer", () => {
        cy.v2CustomerCreateCall(globalState, fixtures.customerCreate);
      });

      it("saved payment method list is empty before the flow", () => {
        cy.v2ListSavedPMCall(globalState, 0);
      });

      it("create persistent payment method session", () => {
        cy.v2PmSessionCreateCall(
          globalState,
          fixtures.paymentMethodSessionCreate
        );
      });

      it("confirm session with card and customer acceptance", () => {
        cy.v2PmSessionConfirmCall(
          globalState,
          fixtures.payThenVaultPmsConfirm,
          expectedEnrichedCard
        );
      });

      it("saved payment method list is still empty after the confirm", () => {
        // The confirmed card lives in redis only — the promotion happens at
        // payment acknowledgement, not at confirm
        cy.v2ListSavedPMCall(globalState, 0);
      });

      it("pay with the ptv token", () => {
        const data = ptvSaveCardData();
        cy.paymentWithSavedPMCall(
          globalState,
          ptvPaymentBody(data.Request),
          true,
          {
            expectedStatus: data.Response.body.status,
            expectedAmountReceived: data.Request.amount,
          }
        );
      });

      it("card is promoted to the vault after the payment", () => {
        cy.v2ListSavedPMCall(globalState, 1, expectedPromotedPm(1200));
      });
    }
  );

  context("Duplicate card for the same customer - fingerprint dedupe", () => {
    it("create second persistent payment method session", () => {
      cy.v2PmSessionCreateCall(
        globalState,
        fixtures.paymentMethodSessionCreate
      );
    });

    it("confirm session with the same card and customer acceptance", () => {
      // Same card number with a different holder name — the dedupe is by
      // vault fingerprint, not by holder name
      const duplicateCardConfirm = {
        ...fixtures.payThenVaultPmsConfirm,
        payment_method_data: {
          card: {
            ...fixtures.payThenVaultPmsConfirm.payment_method_data.card,
            card_holder_name: "Duplicate Card",
          },
        },
      };
      cy.v2PmSessionConfirmCall(
        globalState,
        duplicateCardConfirm,
        expectedEnrichedCard
      );
    });

    it("pay the duplicate amount with the new ptv token", () => {
      const data = ptvSaveCardData();
      cy.paymentWithSavedPMCall(
        globalState,
        ptvPaymentBody(data.Request, { amount: 1300 }),
        true,
        {
          expectedStatus: data.Response.body.status,
          expectedAmountReceived: 1300,
        }
      );
    });

    it("fingerprint dedupe keeps a single saved payment method", () => {
      // Count stays 1 and the connector token's authorized amount moves to
      // the latest payment
      cy.v2ListSavedPMCall(globalState, 1, expectedPromotedPm(1300));
    });
  });

  context("Customer without acceptance - card is never promoted", () => {
    it("create customer", () => {
      cy.v2CustomerCreateCall(globalState, fixtures.customerCreate);
    });

    it("create persistent payment method session", () => {
      cy.v2PmSessionCreateCall(
        globalState,
        fixtures.paymentMethodSessionCreate
      );
    });

    it("confirm session with card and no customer acceptance", () => {
      cy.v2PmSessionConfirmCall(
        globalState,
        Cypress._.omit(fixtures.payThenVaultPmsConfirm, "customer_acceptance"),
        expectedEnrichedCard
      );
    });

    it("pay without customer acceptance", () => {
      const data = ptvSaveCardData();
      cy.paymentWithSavedPMCall(
        globalState,
        Cypress._.omit(
          ptvPaymentBody(data.Request, { amount: 1400 }),
          "customer_acceptance"
        ),
        true,
        {
          expectedStatus: data.Response.body.status,
          expectedAmountReceived: 1400,
        }
      );
    });

    it("saved payment method list stays empty", () => {
      // Acceptance is the promotion gate — the payment succeeded but the
      // card is never promoted
      cy.v2ListSavedPMCall(globalState, 0);
    });
  });

  context("Guest with no acceptance - volatile token payment", () => {
    it("create guest volatile payment method session", () => {
      // Guest flows must not attach a customer — clear the customer from the
      // preceding contexts so the session and the payment omit customer_id
      globalState.set("customerId", undefined);
      cy.v2PmSessionCreateCall(globalState, {
        ...fixtures.paymentMethodSessionCreate,
        storage_type: "volatile",
      });
    });

    it("confirm guest session with card and no customer acceptance", () => {
      cy.v2PmSessionConfirmCall(
        globalState,
        Cypress._.omit(fixtures.payThenVaultPmsConfirm, "customer_acceptance"),
        expectedEnrichedCard
      );
    });

    it("pay as a guest with the ptv token", () => {
      const data = ptvSaveCardData();
      cy.paymentWithSavedPMCall(
        globalState,
        Cypress._.omit(
          ptvPaymentBody(data.Request, { amount: 1100 }),
          "customer_acceptance",
          "setup_future_usage"
        ),
        true,
        {
          expectedStatus: data.Response.body.status,
          expectedAmountReceived: 1100,
        }
      );
    });

    it("token details are retrievable for the volatile payment method", () => {
      cy.v2GetPMFromTokenCall(globalState);
    });
  });

  context("Guest with acceptance - acknowledgement is rejected", () => {
    it("create guest volatile payment method session", () => {
      globalState.set("customerId", undefined);
      cy.v2PmSessionCreateCall(globalState, {
        ...fixtures.paymentMethodSessionCreate,
        storage_type: "volatile",
      });
    });

    it("confirm guest session with card and customer acceptance", () => {
      cy.v2PmSessionConfirmCall(
        globalState,
        fixtures.payThenVaultPmsConfirm,
        expectedEnrichedCard
      );
    });

    it("pay as a guest with the ptv token", () => {
      const data = ptvSaveCardData();
      cy.paymentWithSavedPMCall(
        globalState,
        Cypress._.omit(
          ptvPaymentBody(data.Request, { amount: 1150 }),
          "customer_acceptance",
          "setup_future_usage"
        ),
        true,
        {
          expectedStatus: data.Response.body.status,
          expectedAmountReceived: 1150,
        }
      );
    });

    it("token details are retrievable for the volatile payment method", () => {
      cy.v2GetPMFromTokenCall(globalState);
    });

    it("guest payment method acknowledgement is rejected with 404 HE_02", () => {
      // The guest card was never promoted, so the PM id from the token
      // details has no DB row — the acknowledgement must be rejected
      cy.v2UpdateSavedPMCall(globalState, fixtures.payThenVaultPmsAcknowledge, {
        status: 404,
        code: "HE_02",
        message: "Payment method does not exist in our records",
      });
    });
  });
});
