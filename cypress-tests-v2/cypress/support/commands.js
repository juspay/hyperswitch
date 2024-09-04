// ***********************************************
// This example commands.js shows you how to
// create various custom commands and overwrite
// existing commands.
//
// For more comprehensive examples of custom
// commands please read more here:
// https://on.cypress.io/custom-commands
// ***********************************************
//
//
// -- This is a parent command --
// Cypress.Commands.add('login', (email, password) => { ... })
//
//
// -- This is a child command --
// Cypress.Commands.add('drag', { prevSubject: 'element'}, (subject, options) => { ... })
//
//
// -- This is a dual command --
// Cypress.Commands.add('dismiss', { prevSubject: 'optional'}, (subject, options) => { ... })
//
//
// -- This will overwrite an existing command --
// Cypress.Commands.overwrite('visit', (originalFn, url, options) => { ... })

//  cy.task can only be used in support files (spec files or commands file)
function logRequestId(xRequestId) {
  if (xRequestId) {
    cy.task("cli_log", "x-request-id: " + xRequestId);
  } else {
    cy.task("cli_log", "x-request-id is unavailable in the response headers");
  }
}

// Organization API calls
Cypress.Commands.add(
  "organizationCreateCall",
  (organizationCreateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const url = `${base_url}/v2/organization`;

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
      },
      body: organizationCreateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body)
          .to.have.property("organization_id")
          .and.to.include("org_")
          .and.to.be.a("string").and.not.be.empty;
        globalState.set("organizationId", response.body.organization_id);
        cy.task("setGlobalState", globalState.data);
        expect(response.body).to.have.property("metadata").and.to.equal(null);
      } else {
        // to be updated
        throw new Error(
          `Organization create call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add("organizationRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const organization_id = globalState.get("organizationId");
  const url = `${base_url}/v2/organization/${organization_id}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body)
        .to.have.property("organization_id")
        .and.to.include("org_")
        .and.to.be.a("string").and.not.be.empty;
      globalState.set("organizationId", response.body.organization_id);
      cy.task("setGlobalState", globalState.data);
      expect(response.body.organization_name)
        .to.have.include("Hyperswitch")
        .and.to.be.a("string").and.not.be.empty;
    } else {
      // to be updated
      throw new Error(
        `Organization create call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add(
  "organizationUpdateCall",
  (organizationUpdateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const organization_id = globalState.get("organizationId");
    const url = `${base_url}/v2/organization/${organization_id}`;

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
      },
      body: organizationUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body)
          .to.have.property("organization_id")
          .and.to.include("org_")
          .and.to.be.a("string").and.not.be.empty;
        globalState.set("organizationId", response.body.organization_id);
        cy.task("setGlobalState", globalState.data);
        expect(response.body).to.have.property("metadata").and.to.be.a("object")
          .and.not.be.empty;
      } else {
        // to be updated
        throw new Error(
          `Organization create call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);

// Merchant account API calls
Cypress.Commands.add(
  "merchantAccountCreateCall",
  (merchantAccountCreateBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);
Cypress.Commands.add(
  "merchantAccountRetrieveCall",
  (merchantAccountRetrieveBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);
Cypress.Commands.add(
  "merchantAccountUpdateCall",
  (merchantAccountUpdateBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);

// Business profile API calls
Cypress.Commands.add(
  "businessProfileCreateCall",
  (businessProfileCreateBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);
Cypress.Commands.add(
  "businessProfileRetrieveCall",
  (businessProfileRetrieveBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);
Cypress.Commands.add(
  "businessProfileUpdateCall",
  (businessProfileUpdateBody, globalState) => {
    cy.request({}).then((response) => {});
  }
);
