import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Saved payment method billing vs request billing", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Saved billing is only attached when the modular payment method service resolves the
      // token, and the config is toggled through superposition, so the spec needs both
      specShouldSkip =
        utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.SAVED_PM_BILLING_PREFERENCE
        ) ||
        !globalState.get("pmServiceUrl") ||
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken");
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup configs + flush global state", () => {
    cy.deleteSuperpositionConfig(globalState, {
      provider_merchant_id: globalState.get("merchantId"),
      processor_merchant_id: globalState.get("merchantId"),
    });
    cy.task("setGlobalState", globalState.data);
  });

  context("Config off: saved billing persisted at create is used", () => {
    it("Create Customer", () => {
      cy.customerCreateCall(globalState, fixtures.customerCreate);
    });

    it("Create with saved token, confirm with request billing", () => {
      cy.savedPmBillingCreateThenConfirmTest(
        globalState,
        "MB-Manitoba",
        "MB",
        "MB-Manitoba"
      );
    });
  });

  context(
    "Config on: request billing replaces saved billing persisted at create",
    () => {
      it("Create Customer", () => {
        cy.customerCreateCall(globalState, fixtures.customerCreate);
      });

      it("Set should_prefer_request_billing_over_saved_pm_billing=true", () => {
        cy.setSuperpositionConfig(
          globalState,
          "payments.should_prefer_request_billing_over_saved_pm_billing",
          true,
          {
            provider_merchant_id: globalState.get("merchantId"),
            processor_merchant_id: globalState.get("merchantId"),
          }
        );
      });

      it("Create with saved token, confirm with request billing", () => {
        cy.savedPmBillingCreateThenConfirmTest(
          globalState,
          "MB-Manitoba",
          "MB",
          "MB"
        );
      });
    }
  );
});
