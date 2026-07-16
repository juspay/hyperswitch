import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Utilities/ConnectorOnboarding";

let globalState;

describe("Connector Onboarding", () => {
  before("setup user JWT and PayPal connector", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      cy.connectorOnboardingBootstrap(globalState);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Tests", () => {
    it("should reset connector onboarding tracking ID", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(
        { ...data.Request, connector_id: mcaId },
        data,
        globalState
      );
    });

    it("should reset tracking ID a second time", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(
        { ...data.Request, connector_id: mcaId },
        data,
        globalState
      );
    });
  });

  context("PayPal Credential Tests", () => {
    it("action_url returns action_url or IR_06 depending on environment", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      cy.connectorOnboardingActionUrl(
        { ...data.Request, connector_id: mcaId },
        data,
        globalState
      );
    });

    it("sync returns status or error depending on environment", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const profileId = globalState.get("profileId");
      const data = connectorDetails.ConnectorOnboarding.Sync;
      cy.connectorOnboardingSync(
        { ...data.Request, connector_id: mcaId, profile_id: profileId },
        data,
        globalState
      );
    });
  });

  context("Negative Tests", () => {
    it("should fail action_url with non-existent MCA", () => {
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      cy.connectorOnboardingActionUrl(
        {
          ...data.Request,
          connector_id: "mca_nonexistent_00000000000",
        },
        {
          ...data,
          Response: {
            status: 404,
            body: { error: { type: "invalid_request" } },
          },
        },
        globalState
      );
    });

    it("should fail reset_tracking_id with non-existent MCA", () => {
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      cy.connectorOnboardingResetTrackingId(
        {
          connector: "paypal",
          connector_id: "mca_nonexistent_00000000000",
        },
        {
          ...data,
          Response: {
            status: 404,
            body: { error: { type: "invalid_request" } },
          },
        },
        globalState
      );
    });

    it("should fail action_url with invalid connector type", () => {
      const mcaId = globalState.get("paypalConnectorId");
      cy.connectorOnboardingActionUrl(
        {
          connector: "stripe",
          connector_id: mcaId,
          return_url: "https://example.com/callback",
        },
        {
          Response: {
            status: 422,
            body: { error: { type: "invalid_request" } },
          },
        },
        globalState
      );
    });
  });

  context("Edge Case Tests", () => {
    it("should handle multiple sequential reset_tracking_id calls", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ResetTrackingId;
      const req = { ...data.Request, connector_id: mcaId };
      cy.connectorOnboardingResetTrackingId(req, data, globalState)
        .then(() =>
          cy.connectorOnboardingResetTrackingId(req, data, globalState)
        )
        .then(() =>
          cy.connectorOnboardingResetTrackingId(req, data, globalState)
        );
    });

    it("should handle action_url with special characters in return_url", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      cy.connectorOnboardingActionUrl(
        {
          ...data.Request,
          connector_id: mcaId,
          return_url:
            "https://example.com/callback?redirect=true&source=paypal",
        },
        data,
        globalState
      );
    });
  });
});
