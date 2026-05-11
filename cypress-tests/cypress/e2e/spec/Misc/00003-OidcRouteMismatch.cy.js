import State from "../../../utils/State";

let globalState;

describe("OIDC Route Mismatch - SAIAAAAAA-181", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("OIDC Discovery Document", () => {
    it("should return valid discovery document", () => {
      cy.oidcDiscoveryCallTest(globalState);
    });
  });

  context("OIDC Route Consistency", () => {
    it("should return 404 for /oauth2/authorize (BUG: advertised but not registered)", () => {
      cy.oidcRouteCheckCallTest(globalState, "/oauth2/authorize", 404);
      cy.task(
        "cli_log",
        "BUG CONFIRMED: /oauth2/authorize returns 404 (advertised in discovery but not implemented)"
      );
    });

    it("should respond at /oidc/authorize (actual registered route)", () => {
      const baseUrl = globalState.get("baseUrl");
      const path = `/oidc/authorize?client_id=test&redirect_uri=http://localhost/callback&scope=openid&response_type=code&state=test-state&nonce=test-nonce`;
      cy.oidcRouteCheckCallTest(globalState, path, [
        200, 301, 302, 307, 308, 400, 401, 403,
      ]);
    });
  });

  context("OIDC Supporting Endpoints", () => {
    it("should return JWKS at /oauth2/jwks", () => {
      cy.oidcJwksCallTest(globalState);
    });

    it("should have token endpoint at /oauth2/token", () => {
      cy.oidcTokenEndpointProbeCallTest(globalState);
    });
  });
});
