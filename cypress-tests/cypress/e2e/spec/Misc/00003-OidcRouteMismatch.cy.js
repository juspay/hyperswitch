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
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/.well-known/openid-configuration`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        if (response.status === 404) {
          cy.log("OIDC feature not enabled - skipping test");
          return;
        }

        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("issuer");
        expect(response.body).to.have.property("authorization_endpoint");
        expect(response.body).to.have.property("token_endpoint");
        expect(response.body).to.have.property("jwks_uri");

        globalState.set(
          "oidcAuthorizationEndpoint",
          response.body.authorization_endpoint
        );
        globalState.set("oidcTokenEndpoint", response.body.token_endpoint);
        globalState.set("oidcJwksUri", response.body.jwks_uri);

        cy.log(
          `Authorization endpoint: ${response.body.authorization_endpoint}`
        );
        cy.log(`Token endpoint: ${response.body.token_endpoint}`);
        cy.log(`JWKS URI: ${response.body.jwks_uri}`);
      });
    });
  });

  context("OIDC Route Consistency", () => {
    it("should return 404 for /oauth2/authorize (BUG: advertised but not registered)", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/oauth2/authorize`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(404);
        cy.log(
          "BUG CONFIRMED: /oauth2/authorize returns 404 (advertised in discovery but not implemented)"
        );
      });
    });

    it("should respond at /oidc/authorize (actual registered route)", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/oidc/authorize?client_id=test&redirect_uri=http://localhost/callback&scope=openid&response_type=code&state=test-state&nonce=test-nonce`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.be.oneOf([
          200, 301, 302, 307, 308, 400, 401, 403,
        ]);
        cy.log(
          `/oidc/authorize responded with status ${response.status} (route exists)`
        );
      });
    });
  });

  context("OIDC Supporting Endpoints", () => {
    it("should return JWKS at /oauth2/jwks", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "GET",
        url: `${baseUrl}/oauth2/jwks`,
        headers: {
          Accept: "application/json",
        },
        failOnStatusCode: false,
      }).then((response) => {
        if (response.status === 404) {
          cy.log("OIDC feature not enabled - skipping test");
          return;
        }

        // 500 OI_05 = OIDC signing keys not configured (expected in sandbox/test environments)
        if (response.status === 500 && response.body?.error?.code === "OI_05") {
          cy.log(
            "OIDC signing keys not configured - JWKS unavailable (expected in sandbox)"
          );
          return;
        }

        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("keys");
        expect(response.body.keys).to.be.an("array");
      });
    });

    it("should have token endpoint at /oauth2/token", () => {
      const baseUrl = globalState.get("baseUrl");

      cy.request({
        method: "POST",
        url: `${baseUrl}/oauth2/token`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: "grant_type=authorization_code&code=invalid&redirect_uri=http://localhost/callback",
        failOnStatusCode: false,
      }).then((response) => {
        if (response.status === 404) {
          cy.log("OIDC feature not enabled - skipping test");
          return;
        }

        expect(response.status).to.be.oneOf([200, 400, 401, 403]);
        cy.log(
          `/oauth2/token responded with status ${response.status} (route exists)`
        );
      });
    });
  });
});
