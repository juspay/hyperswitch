import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

// Customer of the run, restored on teardown so that specs running after this
// one keep using it.
let previousCustomerId;

const configKey = "payments.use_merchant_reference_id_as_customer_id";

// The config is resolved with the organization and provider merchant
// dimensions (DimensionsWithProviderMerchantIdAndOrgId in
// dimension_config.rs); a context scoped by processor_merchant_id never
// matches.
const configContext = () => ({
  organization_id: globalState.get("organizationId"),
  provider_merchant_id: globalState.get("merchantId"),
});

/*
merchant_reference_id as customer_id (PR #14589).

Customers created through the modular service's /v1/customers get a global id
(0a_cus_...) and keep the merchant's merchant_reference_id. With
payments.use_merchant_reference_id_as_customer_id on, a v1 payment that sends
that merchant_reference_id as customer_id resolves to the existing customer
instead of creating a new one, and an unknown customer_id is no longer created
implicitly.
*/

describe("merchant_reference_id as customer_id", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      previousCustomerId = globalState.get("customerId");

      // Without credentials, setSuperpositionConfig silently no-ops and every
      // case below would run with the config off.
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping MerchantReferenceIdAsCustomerId spec"
        );
        specShouldSkip = true;
      }

      // The customer with a merchant_reference_id is created on the modular
      // payment method service (PM_SERVICE_URL, see utils/State.js).
      if (!globalState.get("pmServiceUrl")) {
        cy.task(
          "cli_log",
          "PM_SERVICE_URL not set — skipping MerchantReferenceIdAsCustomerId spec"
        );
        specShouldSkip = true;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("reset config, restore customer + flush global state", () => {
    if (!specShouldSkip) {
      cy.setSuperpositionConfig(globalState, configKey, false, configContext());
      globalState.set("customerId", previousCustomerId);
    }
    cy.task("setGlobalState", globalState.data);
  });

  it("create customer with merchant_reference_id on the modular service", () => {
    cy.customerCreateCall(globalState, fixtures.customerCreate);
    cy.then(() => {
      globalState.set("globalCustomerId", globalState.get("customerId"));
    });
  });

  it("enable use_merchant_reference_id_as_customer_id", () => {
    cy.setSuperpositionConfig(globalState, configKey, true, configContext());
    // With the config on, an unknown customer_id is no longer created
    // implicitly, so the polled payments start returning 404
    cy.waitForConfigPropagation(globalState, 404, configKey);
  });

  it("payment by merchant_reference_id resolves to the existing customer", () => {
    globalState.set(
      "customerId",
      globalState.get("customerMerchantReferenceId")
    );
    // The response must carry the customer's global id, not the reference id
    // that was sent
    const data = Cypress._.cloneDeep(
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ]
    );
    data.Response.body.customer_id = globalState.get("globalCustomerId");

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    // Retrieve the stored payment against the same expected body: it must
    // still be linked to the customer's global id, not to the reference id
    // that was sent.
    cy.retrievePaymentCallTest({ globalState, data });
  });

  it("unknown customer_id is not created implicitly", () => {
    globalState.set("customerId", `unknown_reference_${Date.now()}`);
    // Same contract as block_implicit_customer_creation: 404 HE_02
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "BlockImplicitCustomerCreationBlocked"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );
  });
});
