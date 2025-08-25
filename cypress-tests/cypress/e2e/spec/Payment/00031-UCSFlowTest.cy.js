import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("UCS Flow Testing", () => {
  const UCS_SUPPORTED_CONNECTORS = ["authorizedotnet"];
  const UCS_REQUEST_NAMES = [
    "UCSZeroAuthMandate",
    "UCSConfirmMandate",
    "UCSRecurringPayment",
    "No3DSAutoCapture",
    "No3DSManualCapture",
  ];

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  const currentConnector = Cypress.env("CYPRESS_CONNECTOR");
  const isUCSSupported = UCS_SUPPORTED_CONNECTORS.includes(currentConnector);

  if (isUCSSupported) {
    context(`UCS Tests for ${currentConnector.toUpperCase()}`, () => {
      let shouldContinue = true;
      let testableRequests = [];

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      before("load connector config", () => {
        cy.task("cli_log", "=== UCS Flow Testing Started ===");
        cy.task("cli_log", `Testing connector: ${currentConnector}`);

        // Load connector configuration using standard pattern
        const config = getConnectorDetails(currentConnector).card_pm;
        const connectorConfig = { card_pm: config };

        if (!connectorConfig?.card_pm) {
          throw new Error(`Failed to load configuration for connector: ${currentConnector}`);
        }

        const allRequests = Object.keys(connectorConfig.card_pm);
        testableRequests = allRequests.filter((requestType) =>
          UCS_REQUEST_NAMES.includes(requestType)
        );

        // Log coverage information
        cy.task("cli_log", `📊 Total requests available in ${currentConnector}.js: ${allRequests.length}`);
        cy.task("cli_log", `✅ UCS-compatible requests found: ${testableRequests.length}`);
        cy.task("cli_log", `📝 Testable requests: ${testableRequests.join(", ")}`);
        cy.task("cli_log", `📈 Test Coverage: ${testableRequests.length}/${allRequests.length} (${((testableRequests.length / allRequests.length) * 100).toFixed(1)}%)`);

        if (testableRequests.length === 0) {
          throw new Error(`No UCS-compatible requests found for connector: ${currentConnector}`);
        }
      });

      it("should setup UCS environment", () => {
        cy.setupUCSEnvironment({ ...fixtures, payment_methods_enabled }, globalState, currentConnector);
        if (shouldContinue) shouldContinue = true;
      });

      // Dynamic test generation - following pattern from other test files
      UCS_REQUEST_NAMES.forEach((requestType) => {
        it(`should test ${requestType}`, () => {
          // Skip test if request not available for this connector
          if (!testableRequests.includes(requestType)) {
            cy.task("cli_log", `Skipping ${requestType} - not available for ${currentConnector}`);
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][requestType];

          if (!data?.Request || !data?.Response) {
            throw new Error(`Configuration missing for: ${requestType}`);
          }

          cy.task("cli_log", `Testing ${requestType}`);

          if (requestType.includes("MIT") || requestType.includes("Repeat")) {
            cy.task("cli_log", `Skipping ${requestType} - requires existing mandate setup`);
            return;
          }

          if (requestType === "UCSZeroAuthMandate") {
            // Get full connector config for sequential flow
            const connectorConfig = { card_pm: getConnectorDetails(currentConnector).card_pm };
            cy.executeUCSSequentialFlow(connectorConfig, {}, currentConnector, globalState);
          } else {
            // Individual UCS request test
            cy.createUCSPayment(requestType, currentConnector, globalState, data.Request)
              .then((response) => {
                cy.validateUCSResponse(response, data.Response, requestType)
                  .then((result) => {
                    if (!result.success) {
                      shouldContinue = false;
                      throw new Error(`UCS Test Failed - ${result.error}`);
                    }
                  });
              });
          }

          // Use standard pattern for continuation
          if (shouldContinue) shouldContinue = utils.should_continue_further(data);
        });
      });

      after("cleanup UCS configurations", () => {
        cy.cleanupUCSConfigs(globalState, currentConnector);
        cy.task("cli_log", `All UCS tests completed for ${currentConnector}!`);
      });
    });
  } else {
    context(`UCS Tests - Skipped for ${currentConnector}`, () => {
      it("should skip UCS tests for unsupported connector", () => {
        cy.task("cli_log", `Connector ${currentConnector} is not supported for UCS tests`);
        cy.task("cli_log", `Supported UCS connectors: ${UCS_SUPPORTED_CONNECTORS.join(", ")}`);
      });
    });
  }

  after(() => {
    cy.task("cli_log", "\n=== UCS Flow Testing Completed ===");
  });
});