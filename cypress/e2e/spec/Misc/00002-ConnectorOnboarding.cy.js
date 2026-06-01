import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Utilities/ConnectorOnboarding";

let globalState;

describe("Connector Onboarding", () => {
  before("seed global state and create merchant account", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Check if merchant account already exists
      if (!globalState.get("merchantId") || !globalState.get("apiKey")) {
        // Create merchant account using admin API key
        return cy
          .merchantCreateCallTest(fixtures.merchantCreateBody, globalState)
          .then(() => {
            // Create merchant API key for connector creation
            return cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
          });
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Tests", () => {
    it("should retrieve connector onboarding action URL for PayPal", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      cy.connectorOnboardingActionUrl(
        data.Request,
        data,
        globalState
      );
    });

    it("should sync connector onboarding status", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      cy.connectorOnboardingSync(
        data.Request,
        data,
        globalState
      );
    });

    it("should reset connector onboarding tracking ID", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(
        data.Request,
        data,
        globalState
      );
    });

    it("should get a different tracking ID after reset", () => {
      // Store current tracking ID
      const previousTrackingId = globalState.get("onboardingTrackingId");
      
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(
        data.Request,
        data,
        globalState
      ).then(() => {
        // Verify the tracking ID was changed
        const newTrackingId = globalState.get("onboardingTrackingId");
        expect(newTrackingId).to.not.equal(previousTrackingId);
      });
    });
  });

  context("Negative Tests", () => {
    it("should fail with invalid connector ID", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const invalidRequest = {
        ...data.Request,
        connector_id: "invalid_connector_xyz",
      };
      cy.connectorOnboardingActionUrl(invalidRequest, data, globalState);
    });

    it("should fail sync without existing connector integration", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      const requestWithoutConnector = {
        ...data.Request,
        connector_id: "nonexistent_paypal",
      };
      cy.connectorOnboardingSync(requestWithoutConnector, data, globalState);
    });

    it("should fail with missing required fields", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const emptyRequest = {};
      cy.connectorOnboardingActionUrl(emptyRequest, data, globalState);
    });
  });

  context("Edge Case Tests", () => {
    it("should handle action URL request with special characters in connector name", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const requestWithSpecialChars = {
        ...data.Request,
        connector_id: "paypal_test_v1.0",
      };
      cy.connectorOnboardingActionUrl(requestWithSpecialChars, data, globalState);
    });

    it("should handle multiple sequential resets", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      
      // First reset
      cy.connectorOnboardingResetTrackingId(data.Request, data, globalState)
        .then(() => {
          const firstId = globalState.get("onboardingTrackingId");
          
          // Second reset
          return cy.connectorOnboardingResetTrackingId(data.Request, data, globalState);
        })
        .then(() => {
          const secondId = globalState.get("onboardingTrackingId");
          
          // Third reset
          return cy.connectorOnboardingResetTrackingId(data.Request, data, globalState);
        })
        .then(() => {
          const thirdId = globalState.get("onboardingTrackingId");
          expect(thirdId).to.not.be.null;
          expect(thirdId).to.be.a("string");
        });
    });

    it("should handle sync call with empty onboarding state", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      cy.connectorOnboardingSync(data.Request, data, globalState);
    });
  });
});
