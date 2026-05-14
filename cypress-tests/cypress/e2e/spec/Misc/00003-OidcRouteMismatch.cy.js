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

  context("OIDC Advertised Endpoint Reachability", () => {
    it("should document discovery-advertised authorization endpoint returning 404 (route mismatch bug)", () => {
      cy.oidcAdvertisedAuthorizeRouteCheck(globalState);
    });

    it("should respond at /oidc/authorize (actual registered route)", () => {
      cy.oidcAuthorizeRouteCheck(globalState);
    });
  });

  context("OIDC Supporting Endpoints", () => {
    it("should document JWKS endpoint returning 200 with keys array or 500 OI_05", () => {
      cy.oidcJwksCallTest(globalState);
    });

    it("should have token endpoint at /oauth2/token", () => {
      cy.oidcTokenEndpointProbeCallTest(globalState);
    });
  });
});
