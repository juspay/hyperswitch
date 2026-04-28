import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

describe("Authentication Service Eligibility", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Org enabled, Merchant enabled - org takes precedence (3DS enabled)",
    () => {
      it("should set org config to true", () => {
        const orgId = globalState.get("organizationId");
        const key = `authentication_service_eligible_${orgId}`;
        cy.setConfigs(globalState, key, "true", "CREATE");
      });

      it("should set merchant config to true", () => {
        const merchantId = globalState.get("merchantId");
        const key = `authentication_service_eligible_${merchantId}`;
        cy.setConfigs(globalState, key, "true", "CREATE");
      });

      it("should confirm 3DS payment with org and merchant both enabled", () => {
        const data =
          connectorDetails.auth_service_eligibility.OrgEnabledMerchantEnabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup org and merchant configs", () => {
        const orgId = globalState.get("organizationId");
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${orgId}`,
          "true",
          "DELETE"
        );
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${merchantId}`,
          "true",
          "DELETE"
        );
      });
    }
  );

  context(
    "Org enabled, Merchant disabled - org takes precedence (3DS enabled)",
    () => {
      it("should set org config to true", () => {
        const orgId = globalState.get("organizationId");
        const key = `authentication_service_eligible_${orgId}`;
        cy.setConfigs(globalState, key, "true", "CREATE");
      });

      it("should set merchant config to false", () => {
        const merchantId = globalState.get("merchantId");
        const key = `authentication_service_eligible_${merchantId}`;
        cy.setConfigs(globalState, key, "false", "CREATE");
      });

      it("should confirm 3DS payment with org overriding merchant", () => {
        const data =
          connectorDetails.auth_service_eligibility.OrgEnabledMerchantDisabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup org and merchant configs", () => {
        const orgId = globalState.get("organizationId");
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${orgId}`,
          "true",
          "DELETE"
        );
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${merchantId}`,
          "false",
          "DELETE"
        );
      });
    }
  );

  context(
    "Org disabled, Merchant enabled - org takes precedence (3DS disabled)",
    () => {
      it("should set org config to false", () => {
        const orgId = globalState.get("organizationId");
        const key = `authentication_service_eligible_${orgId}`;
        cy.setConfigs(globalState, key, "false", "CREATE");
      });

      it("should set merchant config to true", () => {
        const merchantId = globalState.get("merchantId");
        const key = `authentication_service_eligible_${merchantId}`;
        cy.setConfigs(globalState, key, "true", "CREATE");
      });

      it("should confirm payment with no_three_ds when org overrides merchant", () => {
        const data =
          connectorDetails.auth_service_eligibility.OrgDisabledMerchantEnabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup org and merchant configs", () => {
        const orgId = globalState.get("organizationId");
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${orgId}`,
          "false",
          "DELETE"
        );
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${merchantId}`,
          "true",
          "DELETE"
        );
      });
    }
  );

  context("Org disabled, Merchant disabled - both deny (3DS disabled)", () => {
    it("should set org config to false", () => {
      const orgId = globalState.get("organizationId");
      const key = `authentication_service_eligible_${orgId}`;
      cy.setConfigs(globalState, key, "false", "CREATE");
    });

    it("should set merchant config to false", () => {
      const merchantId = globalState.get("merchantId");
      const key = `authentication_service_eligible_${merchantId}`;
      cy.setConfigs(globalState, key, "false", "CREATE");
    });

    it("should confirm payment with no_three_ds when both configs disabled", () => {
      const data =
        connectorDetails.auth_service_eligibility.OrgDisabledMerchantDisabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    after("cleanup org and merchant configs", () => {
      const orgId = globalState.get("organizationId");
      const merchantId = globalState.get("merchantId");
      cy.setConfigs(
        globalState,
        `authentication_service_eligible_${orgId}`,
        "false",
        "DELETE"
      );
      cy.setConfigs(
        globalState,
        `authentication_service_eligible_${merchantId}`,
        "false",
        "DELETE"
      );
    });
  });

  context(
    "No org config, Merchant enabled - merchant fallback (3DS enabled)",
    () => {
      it("should set merchant config to true", () => {
        const merchantId = globalState.get("merchantId");
        const key = `authentication_service_eligible_${merchantId}`;
        cy.setConfigs(globalState, key, "true", "CREATE");
      });

      it("should confirm 3DS payment with merchant-only config enabled", () => {
        const data =
          connectorDetails.auth_service_eligibility.MerchantOnlyEnabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup merchant config", () => {
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${merchantId}`,
          "true",
          "DELETE"
        );
      });
    }
  );

  context(
    "No org config, Merchant disabled - merchant fallback (3DS disabled)",
    () => {
      it("should set merchant config to false", () => {
        const merchantId = globalState.get("merchantId");
        const key = `authentication_service_eligible_${merchantId}`;
        cy.setConfigs(globalState, key, "false", "CREATE");
      });

      it("should confirm payment with no_three_ds when merchant config disabled", () => {
        const data =
          connectorDetails.auth_service_eligibility.MerchantOnlyDisabled;
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      after("cleanup merchant config", () => {
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `authentication_service_eligible_${merchantId}`,
          "false",
          "DELETE"
        );
      });
    }
  );

  context("No config at all - default behavior", () => {
    it("should confirm 3DS payment with default behavior (no config set)", () => {
      const data = connectorDetails.auth_service_eligibility.NoConfigDefault;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });
  });
});
