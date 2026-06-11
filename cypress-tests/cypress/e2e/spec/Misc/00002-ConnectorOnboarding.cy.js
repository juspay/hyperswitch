import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Utilities/ConnectorOnboarding";

let globalState;

describe("Connector Onboarding", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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
      cy.connectorOnboardingActionUrl(data.Request, data, globalState);
    });

    it("should reset connector onboarding tracking ID", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(data.Request, data, globalState);
    });

    it("should successfully reset tracking ID a second time", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(data.Request, data, globalState);
    });
  });

  context("Negative Tests", () => {
    it("should fail action_url with invalid connector type", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const invalidRequest = {
        ...data.Request,
        connector: "invalid_connector_xyz",
      };
      cy.connectorOnboardingActionUrl(invalidRequest, data, globalState);
    });

    it("should return error for sync without PayPal connector integration", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      cy.connectorOnboardingSync(data.Request, data, globalState);
    });

    it("should fail sync with nonexistent connector", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      const requestWithInvalidConnector = {
        ...data.Request,
        connector: "nonexistent_connector",
      };
      cy.connectorOnboardingSync(requestWithInvalidConnector, data, globalState);
    });

    it("should fail action_url with missing required fields", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const emptyRequest = {};
      cy.connectorOnboardingActionUrl(emptyRequest, data, globalState);
    });
  });

  context("Edge Case Tests", () => {
    it("should handle action URL request with special characters in return_url", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      const requestWithSpecialChars = {
        ...data.Request,
        return_url: "https://example.com/callback?redirect=true&source=paypal",
      };
      cy.connectorOnboardingActionUrl(requestWithSpecialChars, data, globalState);
    });

    it("should handle multiple sequential reset_tracking_id calls", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(data.Request, data, globalState)
        .then(() =>
          cy.connectorOnboardingResetTrackingId(data.Request, data, globalState)
        )
        .then(() =>
          cy.connectorOnboardingResetTrackingId(data.Request, data, globalState)
        );
    });

    it("should handle sync call with minimal request body", () => {
      const data = connectorDetails.ConnectorOnboarding.Sync;
      const minimalRequest = {
        connector: data.Request.connector,
        connector_id: data.Request.connector_id,
      };
      cy.connectorOnboardingSync(minimalRequest, data, globalState);
    });
  });
});
