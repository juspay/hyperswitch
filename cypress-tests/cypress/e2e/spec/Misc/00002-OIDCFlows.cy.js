import State from "../../../utils/State";

let globalState;

describe("OIDC Flows", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("OIDC Discovery Document", () => {
    it("should retrieve OIDC discovery document using GET", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/.well-known/openid-configuration`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        logRequestId(response.headers["x-request-id"]);

        expect(response.status).to.equal(200);
        expect(response.headers["content-type"]).to.include("application/json");
        expect(response.body).to.have.property("issuer");
        expect(response.body).to.have.property("authorization_endpoint");
        expect(response.body).to.have.property("token_endpoint");
        expect(response.body).to.have.property("jwks_uri");
        expect(response.body).to.have.property("response_types_supported");
        expect(response.body).to.have.property("grant_types_supported");
        expect(response.body).to.have.property("scopes_supported");
      });
    });
  });

  context("OIDC JWKS Endpoint", () => {
    it("should retrieve JWKS using GET", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/oauth2/jwks`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        logRequestId(response.headers["x-request-id"]);

        expect(response.status).to.equal(200);
        expect(response.headers["content-type"]).to.include("application/json");
        expect(response.body).to.have.property("keys");
        expect(response.body.keys).to.be.an("array");
      });
    });
  });

  context("OIDC Authorize Endpoint", () => {
    it("should handle authorize request using GET", () => {
      const baseUrl = globalState.get("baseUrl");
      const clientId = globalState.get("merchantId") || "test_client";
      const redirectUri = "https://example.com/callback";
      const state = "test_state_123";
      const nonce = "test_nonce_123";

      cy.request({
        method: "GET",
        url: `${baseUrl}/oidc/authorize`,
        qs: {
          response_type: "code",
          client_id: clientId,
          redirect_uri: redirectUri,
          scope: "openid email profile",
          state: state,
          nonce: nonce,
        },
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        logRequestId(response.headers["x-request-id"]);

        // The authorize endpoint may return 200, 302, or 400 depending on auth state
        // We just verify the GET method works and doesn't return 405 (method not allowed)
        expect(response.status).to.not.equal(405);
        expect(response.status).to.be.oneOf([200, 302, 400, 401]);
      });
    });
  });

  context("OIDC Token Endpoint", () => {
    it("should handle token request using POST", () => {
      const baseUrl = globalState.get("baseUrl");

      // Token endpoint requires client authentication and a valid auth code
      // This test verifies the endpoint accepts POST requests
      cy.request({
        method: "POST",
        url: `${baseUrl}/oauth2/token`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: {
          grant_type: "authorization_code",
          code: "invalid_test_code",
          redirect_uri: "https://example.com/callback",
        },
        failOnStatusCode: false,
      }).then((response) => {
        logRequestId(response.headers["x-request-id"]);

        // Should return 400 or 401 for invalid code, but not 405 (method not allowed)
        expect(response.status).to.not.equal(405);
        expect(response.status).to.be.oneOf([200, 400, 401, 403]);
      });
    });
  });
});
