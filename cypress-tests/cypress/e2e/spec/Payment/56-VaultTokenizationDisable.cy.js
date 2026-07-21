import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("Vault Tokenization Disable", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.VAULT_TOKENIZATION
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Vault tokenization disabled - flag set to true", () => {
    before("set should_disable_vault_tokenization to true", () => {
      const merchantId = globalState.get("merchantId");
      const key = `should_disable_vault_tokenization_${merchantId}`;
      cy.setConfigs(globalState, key, "true", "CREATE");
    });

    it("should confirm external 3DS payment with vault tokenization disabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .vault_tokenization.VaultTokenizationDisabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    after("cleanup vault tokenization config", () => {
      const merchantId = globalState.get("merchantId");
      cy.setConfigs(
        globalState,
        `should_disable_vault_tokenization_${merchantId}`,
        "true",
        "DELETE"
      );
    });
  });

  context("Vault tokenization enabled - flag set to false", () => {
    before("set should_disable_vault_tokenization to false", () => {
      const merchantId = globalState.get("merchantId");
      const key = `should_disable_vault_tokenization_${merchantId}`;
      cy.setConfigs(globalState, key, "false", "CREATE");
    });

    it("should confirm external 3DS payment with vault tokenization enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))
        .vault_tokenization.VaultTokenizationEnabled;
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    after("cleanup vault tokenization config", () => {
      const merchantId = globalState.get("merchantId");
      cy.setConfigs(
        globalState,
        `should_disable_vault_tokenization_${merchantId}`,
        "false",
        "DELETE"
      );
    });
  });
});
