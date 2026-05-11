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
      cy.oidcOauth2AuthorizeRouteCheck(globalState);
    });

    it("should respond at /oidc/authorize (actual registered route)", () => {
      cy.oidcAuthorizeRouteCheck(globalState);
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
