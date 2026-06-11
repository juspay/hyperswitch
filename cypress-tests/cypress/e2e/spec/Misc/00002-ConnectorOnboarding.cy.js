import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Utilities/ConnectorOnboarding";

let globalState;

describe("Connector Onboarding", () => {
  before("setup user JWT and PayPal connector", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      const baseUrl = globalState.get("baseUrl");
      const ts = Date.now();

      // 1. Signup to get TOTP token
      cy.request({
        method: "POST",
        url: `${baseUrl}/user/signup`,
        headers: { "Content-Type": "application/json" },
        body: {
          email: `qa_onboarding_${ts}@example.com`,
          password: "Test@1234",
          name: "QA Test",
          country: "US",
          company_name: "Test Co",
        },
        failOnStatusCode: false,
      }).then((signupResp) => {
        expect(signupResp.status).to.equal(200);
        const totpToken = signupResp.body.token;
        globalState.set("totpToken", totpToken);

        // 2. Terminate 2FA to get full user JWT
        cy.request({
          method: "GET",
          url: `${baseUrl}/user/2fa/terminate?skip_two_factor_auth=true`,
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${totpToken}`,
          },
          failOnStatusCode: false,
        }).then((terminateResp) => {
          expect(terminateResp.status).to.equal(200);
          const userInfoToken = terminateResp.body.token;
          globalState.set("userInfoToken", userInfoToken);

          // Decode JWT to extract merchant_id and profile_id
          const payload = JSON.parse(atob(userInfoToken.split(".")[1]));
          globalState.set("merchantId", payload.merchant_id);
          globalState.set("profileId", payload.profile_id);

          // 3. Create PayPal connector to get MCA ID
          cy.request({
            method: "POST",
            url: `${baseUrl}/account/${payload.merchant_id}/connectors`,
            headers: {
              "Content-Type": "application/json",
              Authorization: `Bearer ${userInfoToken}`,
            },
            body: {
              connector_type: "payment_processor",
              connector_name: "paypal",
              connector_account_details: {
                auth_type: "BodyKey",
                api_key: "test_paypal_key",
                key1: "test_paypal_secret",
              },
              test_mode: true,
              disabled: false,
            },
            failOnStatusCode: false,
          }).then((connectorResp) => {
            expect(connectorResp.status).to.equal(200);
            globalState.set(
              "paypalConnectorId",
              connectorResp.body.merchant_connector_id
            );
          });
        });
      });
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

  context("PayPal Credential Tests (IR_06 in dev env)", () => {
    it("action_url returns IR_06 when PayPal credentials are not configured", () => {
      const mcaId = globalState.get("paypalConnectorId");
      const data = connectorDetails.ConnectorOnboarding.ActionUrl;
      cy.connectorOnboardingActionUrl(
        { ...data.Request, connector_id: mcaId },
        data,
        globalState
      );
    });

    it("sync returns error when no PayPal connector integration exists", () => {
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
        { ...data, Response: { status: 404, body: { error: {} } } },
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
        { ...data, Response: { status: 404, body: { error: {} } } },
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
            body: { error: {} },
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
