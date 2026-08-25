import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

// `authentication_service_eligible` used to be two db configs read in order:
// `authentication_service_eligible_{org_id}` first, and only if that key was
// absent, `authentication_service_eligible_{merchant_id}`. It now resolves from
// superposition (`payments.authentication_service_eligible`, dimensions
// organization_id + processor_merchant_id — see dimension_config.rs), and the
// seeded default `false` wins over the db value, so the `/configs` writes this
// spec used to make no longer have any effect.
//
// NOTE: superposition resolves the *most specific* matching context, so a
// merchant-scoped override now wins over an org-scoped one. That is the inverse
// of the old org-first precedence. The payment assertions below cannot observe
// this: eligibility only decides whether authentication goes through the
// unified authentication service, and both paths return the same API response.
// They are kept as-is and cover the config plumbing, not the precedence.
const ORG_ELIGIBLE_KEY = "payments.authentication_service_eligible";
const STORAGE_KEY =
  "payments.should_store_eligibility_check_data_for_authentication";

const orgContext = () => ({
  organization_id: globalState.get("organizationId"),
});

const merchantContext = () => ({
  organization_id: globalState.get("organizationId"),
  processor_merchant_id: globalState.get("merchantId"),
});

// `should_store_eligibility_check_data_for_authentication` is scoped by
// DimensionsWithProcessorAndProviderMerchantId, not by organization_id.
const storageContext = () => ({
  provider_merchant_id: globalState.get("merchantId"),
  processor_merchant_id: globalState.get("merchantId"),
});

const setOrgEligibility = (value) =>
  cy.setSuperpositionConfig(globalState, ORG_ELIGIBLE_KEY, value, orgContext());

const clearOrgEligibility = () =>
  cy.deleteSuperpositionContext(globalState, orgContext());

const setMerchantEligibility = (value) =>
  cy.setSuperpositionConfig(
    globalState,
    ORG_ELIGIBLE_KEY,
    value,
    merchantContext()
  );

const clearMerchantEligibility = () =>
  cy.deleteSuperpositionContext(globalState, merchantContext());

// Each superposition write costs one polling interval, so the contexts below
// only write what changes relative to the context before them and run in file
// order: the org override is set for the first four, cleared once for the
// merchant-only pair, and the merchant override is cleared for the last.
describe("Authentication Service Eligibility", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.AUTH_SERVICE_ELIGIBILITY
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

  after("cleanup superposition overrides and flush global state", () => {
    cy.deleteSuperpositionContext(globalState, storageContext());
    cy.task("setGlobalState", globalState.data);
  });

  context("Org enabled, merchant enabled", () => {
    before("set org and merchant overrides to true", () => {
      setOrgEligibility(true);
      setMerchantEligibility(true);
    });

    it("should confirm 3DS payment with org and merchant both enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.OrgEnabledMerchantEnabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Org enabled, merchant disabled", () => {
    before("set merchant override to false", () => {
      setMerchantEligibility(false);
    });

    it("should confirm 3DS payment with org enabled and merchant disabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.OrgEnabledMerchantDisabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Org disabled, merchant enabled", () => {
    before("set org override to false and merchant override to true", () => {
      setOrgEligibility(false);
      setMerchantEligibility(true);
    });

    it("should confirm payment with no_three_ds when org is disabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.OrgDisabledMerchantEnabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Org disabled, merchant disabled", () => {
    before("set merchant override to false", () => {
      setMerchantEligibility(false);
    });

    it("should confirm payment with no_three_ds when both overrides disabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.OrgDisabledMerchantDisabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("No org override, merchant enabled", () => {
    before("clear org override and set merchant override to true", () => {
      clearOrgEligibility();
      setMerchantEligibility(true);
    });

    it("should confirm 3DS payment with merchant-only override enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.MerchantOnlyEnabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("No org override, merchant disabled", () => {
    before("set merchant override to false", () => {
      setMerchantEligibility(false);
    });

    it("should confirm payment with no_three_ds when merchant override disabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.MerchantOnlyDisabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("No override at all - default behavior", () => {
    before("clear merchant override", () => {
      clearMerchantEligibility();
    });

    it("should confirm 3DS payment with default behavior (no override set)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .auth_service_eligibility.NoConfigDefault;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });

  context(
    "Eligibility data storage enabled - data stored during UAS 3DS flow",
    () => {
      before("enable eligibility storage override", () => {
        cy.setSuperpositionConfig(
          globalState,
          STORAGE_KEY,
          true,
          storageContext()
        );
      });

      it("should confirm 3DS payment with eligibility storage enabled", () => {
        cy.log(
          "Note: Redis storage of eligibility data cannot be directly asserted via Cypress API"
        );
        const data = getConnectorDetails(globalState.get("connectorId"))
          .auth_service_eligibility.EligibilityStorageEnabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Eligibility data storage disabled - no data stored during UAS 3DS flow",
    () => {
      before("disable eligibility storage override", () => {
        cy.setSuperpositionConfig(
          globalState,
          STORAGE_KEY,
          false,
          storageContext()
        );
      });

      it("should confirm 3DS payment with eligibility storage disabled", () => {
        cy.log(
          "Note: Redis storage of eligibility data cannot be directly asserted via Cypress API"
        );
        const data = getConnectorDetails(globalState.get("connectorId"))
          .auth_service_eligibility.EligibilityStorageDisabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });
    }
  );
});
