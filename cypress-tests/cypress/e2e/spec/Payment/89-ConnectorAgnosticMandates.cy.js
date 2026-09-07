import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

// Original profile/MCA of the run, restored on teardown so that specs running
// after this one do not point at the profile deleted here.
let previousProfileId;
let previousMerchantConnectorId;

// Superposition context for the org level configs set by this spec. Both
// configs are org scoped (see dimension_config.rs), so they share one context
// and are written in a single PUT.
const superpositionContext = () => ({
  organization_id: globalState.get("organizationId"),
});

// `should_perform_sdk_vaulting` is an org level override on top of
// `should_call_pm_modular_service`; when false the SDK skips vaulting. It
// defaults to true, so this spec has to turn it off.
const superpositionOverrides = {
  should_call_pm_modular_service: true,
  should_perform_sdk_vaulting: false,
};

// The modular payment method service is a separate deployment addressed by
// PM_SERVICE_URL (see utils/State.js). It is not provisioned in the standard
// payments run, so the customer and saved card steps below fall back to their
// v1 equivalents when it is absent rather than failing the whole spec.
const modularPmServiceEnabled = () => Boolean(globalState.get("pmServiceUrl"));

// Customer create: /v1/customers on the modular service
const createCustomer = () => {
  if (!modularPmServiceEnabled()) {
    cy.task("cli_log", "PM_SERVICE_URL not set — using v1 customer create");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    return;
  }
  cy.customerCreateCall(globalState, fixtures.customerCreate);
};

// Saved payment method list: /v1/customers/{id}/saved-payment-methods on the
// modular service. Both commands publish `paymentMethodId`/`paymentToken` for
// the payment that follows.
const listSavedPaymentMethods = () => {
  if (!modularPmServiceEnabled()) {
    cy.task("cli_log", "PM_SERVICE_URL not set — using v1 payment method list");
    cy.listCustomerPMByClientSecret(globalState);
    return;
  }
  cy.listSavedPMCall(globalState);
};

// Payment against the saved payment method published by the preceding list.
// The modular list returns no separate token — its `id` (0a_pm_...) is what
// goes in `payment_token`, which is what `useToken: false` sends. The v1
// fallback pays with the real `paymentToken` the v1 list publishes.
const confirmWithSavedPaymentMethod = (data) => {
  if (!modularPmServiceEnabled()) {
    cy.task("cli_log", "PM_SERVICE_URL not set — using v1 save card confirm");
    cy.saveCardConfirmCallTest(
      Cypress._.cloneDeep(fixtures.saveCardConfirmBody),
      data,
      globalState
    );
    return;
  }
  // `paymentWithSavedPMCall` takes no connector config, so the config's Request
  // is merged over the fixture here — otherwise the fixture's amount (200) and
  // its missing `payment_channel` would be sent instead of the config's values
  cy.paymentWithSavedPMCall(
    globalState,
    { ...fixtures.modularPmServicePaymentsCall, ...data?.Request },
    false /* useToken — the payment method id is the token here */
  );
};

/*
TSYS TransIT.

The no 3DS card flows (Visa, Mastercard, Amex) run first, on the profile created
by 03-ConnectorCreate.

The mandate flows then run on a profile of their own: TSYS TransIT does not
return a connector mandate id, so recurring payments are only possible over a
profile with connector agnostic MIT enabled. That makes it unfit for the shared
mandate specs (14-SaveCardFlow, 20-MandatesUsingPMID), hence this spec. For the
same reason the `connector_mandate_id` assertion is skipped for this connector,
see CONNECTOR_LISTS.EXCLUDE.CONNECTOR_MANDATE_ID.

Mandate flow:
- Create a Business Profile and enable connector agnostic MIT on it
- Create a Merchant Connector Account and a Customer on that profile

- Save card [off_session]: CIT with customer_acceptance -> list payment methods
  -> MIT using the payment token

- Mandate using PMID: CIT with customer_acceptance -> MIT using payment_method_id

- Delete the Merchant Connector Account and the Business Profile
*/

describe("TSYS TransIT", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Only tsys_transit; every other connector is covered by the shared
        // specs
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.CONNECTOR_AGNOSTIC_MANDATES
          )
        ) {
          skip = true;
        }

        previousProfileId = globalState.get("profileId");
        previousMerchantConnectorId = globalState.get("merchantConnectorId");
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  // Both configs are org scoped (see dimension_config.rs). The context is left
  // in place after the run rather than removed on teardown.
  before("set the org level configs", () => {
    // Without the organization dimension the overrides would apply to the whole
    // superposition workspace, so they are only set once the org id is known
    if (!globalState.get("organizationId")) {
      cy.task("cli_log", "organizationId not set — skipping org level configs");
      return;
    }

    cy.setSuperpositionConfigs(
      globalState,
      superpositionOverrides,
      superpositionContext()
    );
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - No 3DS auto capture", () => {
    it("Create Customer", () => {
      createCustomer();
    });

    it("Visa - Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureVisa"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureVisa"];

        // The status is only checked when it is passed explicitly,
        // `retrievePaymentCallTest` does not read `data.Response`
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "succeeded",
        });
      });
    });

    it("Mastercard - Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureMastercard"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureMastercard"];

        // The status is only checked when it is passed explicitly,
        // `retrievePaymentCallTest` does not read `data.Response`
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "succeeded",
        });
      });
    });

    it("Amex - Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureAmex"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCaptureAmex"];

        // The status is only checked when it is passed explicitly,
        // `retrievePaymentCallTest` does not read `data.Response`
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "succeeded",
        });
      });
    });
  });

  context("Setup - profile with connector agnostic MIT enabled", () => {
    it("Create business profile", () => {
      utils.createBusinessProfile(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("Enable Connector Agnostic for Business Profile", () => {
      utils.updateBusinessProfile(
        fixtures.businessProfile.bpUpdate,
        true, // is_connector_agnostic_enabled
        false, // collect_billing_address_from_wallet_connector
        false, // collect_shipping_address_from_wallet_connector
        false, // always_collect_billing_address_from_wallet_connector
        false, // always_collect_shipping_address_from_wallet_connector
        globalState
      );
    });

    it("Create merchant connector account", () => {
      utils.createMerchantConnectorAccount(
        "payment_processor",
        fixtures.createConnectorBody,
        globalState,
        payment_methods_enabled
      );
    });
  });

  context("Save card off session - CIT and MIT using payment token", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Customer", () => {
      createCustomer();
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateCitOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateCitOffSession"];

      cy.retrievePaymentCallTest({ globalState, data });

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer using Client Secret", () => {
      listSavedPaymentMethods();
    });

    it("Create Payment Intent for MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer", () => {
      listSavedPaymentMethods();
    });

    it("Confirm No 3DS MIT (Token)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateMitUsingToken"];

      confirmWithSavedPaymentMethod(data);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateMitUsingToken"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Mandate using PMID - CIT and MIT using payment_method_id", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Customer", () => {
      createCustomer();
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdCit"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdCit"];

      cy.retrievePaymentCallTest({ globalState, data });

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS MIT (PMID)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true /* confirm */,
        "automatic",
        globalState,
        true /* connector_agnostic_mit */
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  // Zero auth: a `setup_mandate` CIT for zero amount that only stores the
  // payment method, followed by a real MIT against it. Runs on the connector
  // agnostic profile like the other mandate contexts.
  context("Zero auth mandate - CIT with zero amount and MIT using PMID", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Customer", () => {
      createCustomer();
    });

    it("Confirm No 3DS CIT (Zero Auth)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthConfirmPayment"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        0 /* amount */,
        true /* confirm */,
        "automatic",
        "setup_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthMandate"];

      cy.retrievePaymentCallTest({ globalState, data });

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer", () => {
      listSavedPaymentMethods();
    });

    it("Confirm No 3DS MIT (PMID)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true /* confirm */,
        "automatic",
        globalState,
        true /* connector_agnostic_mit */
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  // Off session CIT per card. TSYS TransIT returns no connector mandate id, so
  // the payment method is only reusable over the connector agnostic profile
  // created above — hence this runs before the teardown, not with the card
  // flows at the top of the spec.
  context("Save card off session CIT - per card", () => {
    it("Create Customer", () => {
      createCustomer();
    });

    it("Visa - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionVisa"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();
      });
    });

    it("Mastercard - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionMastercard"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();
      });
    });

    it("Amex - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionAmex"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();
      });
    });
  });

  // Three off session CITs on three different cards, one of them promoted to
  // the customer's default payment method, then a MIT paid with that card.
  context("Default card - three saved cards and MIT using PMID", () => {
    let shouldContinue = true;

    // Payment method id of the card promoted to default, captured right after
    // its own CIT so the choice does not depend on how the list is ordered.
    let defaultPaymentMethodId;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Customer", () => {
      createCustomer();
    });

    it("Visa - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionVisa"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();

        // This is the card set as default further down
        cy.then(() => {
          defaultPaymentMethodId = globalState.get("paymentMethodId");
        });
      });
    });

    it("Mastercard - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionMastercard"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();
      });
    });

    it("Amex - Create Payment Intent -> Confirm CIT -> List Payment Methods", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

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

      cy.step("Confirm CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSessionAmex"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Payment Method for Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Method for Customer");
          return;
        }
        listSavedPaymentMethods();
      });
    });

    it("Set the Visa card as the default payment method", () => {
      cy.then(() => {
        expect(defaultPaymentMethodId, "captured payment method id").to.not.be
          .undefined;

        // `setDefaultPaymentMethodTest` and `mitUsingPMId` both read
        // `paymentMethodId`, which the Amex list above left pointing elsewhere
        globalState.set("paymentMethodId", defaultPaymentMethodId);

        // The three CITs were confirmed with setup_future_usage off_session,
        // but `confirmCallTest` does not record it the way
        // `citForMandatesCallTest` does, and `mitUsingPMId` reads this to tell
        // whether the connector was downgraded to on_session
        globalState.set("mandateSetupFutureUsage", "off_session");
      });

      cy.setDefaultPaymentMethodTest(globalState);
    });

    it("Confirm No 3DS MIT (PMID) using the default card", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true /* confirm */,
        "automatic",
        globalState,
        true /* connector_agnostic_mit */
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment after MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandatePmIdMit"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Teardown - delete business profile", () => {
    it("Delete merchant connector account", () => {
      cy.connectorDeleteCall(globalState);
    });

    it("Delete business profile", () => {
      cy.deleteBusinessProfileTest(globalState);
    });

    it("Restore previous profile and merchant connector account", () => {
      globalState.set("profileId", previousProfileId);
      globalState.set("merchantConnectorId", previousMerchantConnectorId);
    });
  });
});
