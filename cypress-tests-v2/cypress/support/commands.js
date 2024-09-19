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

import {
  getValueByKey,
  isoTimeTomorrow,
} from "../e2e/configs/Payment/Utils.js";

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
      expect(response.body.organization_name)
        .to.have.include("Hyperswitch")
        .and.to.be.a("string").and.not.be.empty;

      if (organization_id === undefined || organization_id === null) {
        globalState.set("organizationId", response.body.organization_id);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `Organization retrieve call failed with status ${response.status} and message ${response.body.message}`
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
        expect(response.body).to.have.property("metadata").and.to.be.a("object")
          .and.not.be.empty;

        if (organization_id === undefined || organization_id === null) {
          globalState.set("organizationId", response.body.organization_id);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `Organization update call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);

// Merchant account API calls
Cypress.Commands.add(
  "merchantAccountCreateCall",
  (merchantAccountCreateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const organization_id = globalState.get("organizationId");
    const url = `${base_url}/v2/accounts`;

    const merchant_name = merchantAccountCreateBody.merchant_name
      .replaceAll(" ", "")
      .toLowerCase();

    // Update request body
    merchantAccountCreateBody.organization_id = organization_id;

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
      },
      body: merchantAccountCreateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body)
          .to.have.property("id")
          .and.to.include(`${merchant_name}_`)
          .and.to.be.a("string").and.not.be.empty;

        if (base_url.includes("sandbox") || base_url.includes("integ"))
          expect(response.body)
            .to.have.property("publishable_key")
            .and.to.include("pk_snd").and.to.not.be.empty;
        else if (base_url.includes("localhost"))
          expect(response.body)
            .to.have.property("publishable_key")
            .and.to.include("pk_dev").and.to.not.be.empty;

        globalState.set("merchantId", response.body.id);
        globalState.set("publishableKey", response.body.publishable_key);

        cy.task("setGlobalState", globalState.data);
      } else {
        // to be updated
        throw new Error(
          `Merchant create call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add("merchantAccountRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/accounts/${merchant_id}`;

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
      expect(response.body).to.have.property("id").and.to.be.a("string").and.not
        .be.empty;

      if (base_url.includes("sandbox") || base_url.includes("integ"))
        expect(response.body)
          .to.have.property("publishable_key")
          .and.to.include("pk_snd").and.to.not.be.empty;
      else
        expect(response.body)
          .to.have.property("publishable_key")
          .and.to.include("pk_dev").and.to.not.be.empty;

      if (merchant_id === undefined || merchant_id === null) {
        globalState.set("merchantId", response.body.id);
        globalState.set("publishableKey", response.body.publishable_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `Merchant retrieve call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add(
  "merchantAccountUpdateCall",
  (merchantAccountUpdateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const merchant_id = globalState.get("merchantId");
    const url = `${base_url}/v2/accounts/${merchant_id}`;

    const merchant_name = merchantAccountUpdateBody.merchant_name;

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
      },
      body: merchantAccountUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body.id).to.equal(merchant_id);

        if (base_url.includes("sandbox") || base_url.includes("integ"))
          expect(response.body)
            .to.have.property("publishable_key")
            .and.to.include("pk_snd").and.to.not.be.empty;
        else
          expect(response.body)
            .to.have.property("publishable_key")
            .and.to.include("pk_dev").and.to.not.be.empty;
        expect(response.body.merchant_name).to.equal(merchant_name);

        if (merchant_id === undefined || merchant_id === null) {
          globalState.set("merchantId", response.body.id);
          globalState.set("publishableKey", response.body.publishable_key);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `Merchant update call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);

// Business profile API calls
Cypress.Commands.add(
  "businessProfileCreateCall",
  (businessProfileCreateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const merchant_id = globalState.get("merchantId");
    const url = `${base_url}/v2/profiles`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
    };

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
        ...customHeaders,
      },
      body: businessProfileCreateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body.merchant_id).to.equal(merchant_id);
        expect(response.body.id).to.include("pro_").and.to.not.be.empty;
        expect(response.body.profile_name).to.equal(
          businessProfileCreateBody.profile_name
        );

        globalState.set("profileId", response.body.id);

        cy.task("setGlobalState", globalState.data);
      } else {
        // to be updated
        throw new Error(
          `Merchant update call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add("businessProfileRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const profile_id = globalState.get("profileId");
  const url = `${base_url}/v2/profiles/${profile_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.merchant_id).to.equal(merchant_id);
      expect(response.body.id).to.include("pro_").and.to.not.be.empty;

      if (profile_id === undefined || profile_id === null) {
        globalState.set("profileId", response.body.id);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `Merchant update call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add(
  "businessProfileUpdateCall",
  (businessProfileUpdateBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const merchant_id = globalState.get("merchantId");
    const profile_id = globalState.get("profileId");
    const url = `${base_url}/v2/profiles/${profile_id}`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
    };

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
        ...customHeaders,
      },
      body: businessProfileUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body.merchant_id).to.equal(merchant_id);
        expect(response.body.id).to.include("pro_").and.to.not.be.empty;
        expect(response.body.profile_name).to.equal(
          businessProfileUpdateBody.profile_name
        );

        if (profile_id === undefined || profile_id === null) {
          globalState.set("profileId", response.body.id);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `Merchant update call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);

// Merchant Connector Account API calls
// Payments API calls
Cypress.Commands.add(
  "mcaCreateCall",
  (
    connectorLabel,
    connectorName,
    connectorType,
    globalState,
    mcaCreateBody,
    paymentMethodsEnabled
  ) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const merchant_id = globalState.get("merchantId");
    const profile_id = globalState.get("profileId");
    const url = `${base_url}/v2/connector_accounts`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
    };

    // Update request body
    mcaCreateBody.profile_id = profile_id;
    mcaCreateBody.connector_label = connectorLabel;
    mcaCreateBody.connector_name = connectorName;
    mcaCreateBody.connector_type = connectorType;
    mcaCreateBody.payment_methods_enabled = paymentMethodsEnabled;

    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const jsonString = JSON.stringify(jsonContent);
        const key =
          connectorType === "payment_processor"
            ? connectorName
            : `${connectorName}_payout`;
        const authDetails = getValueByKey(jsonString, key);

        mcaCreateBody.connector_account_details =
          authDetails.connector_account_details;

        if (authDetails && authDetails.metadata) {
          createConnectorBody.metadata = {
            ...createConnectorBody.metadata, // Preserve existing metadata fields
            ...authDetails.metadata, // Merge with authDetails.metadata
          };
        }

        cy.request({
          method: "POST",
          url: url,
          headers: {
            "Content-Type": "application/json",
            "api-key": api_key,
            ...customHeaders,
          },
          body: mcaCreateBody,
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(response.body.connector_name).to.equal(connectorName);
            expect(response.body.id).to.include("mca_").and.to.not.be.empty;
            expect(response.body.status).to.equal("active");
            expect(response.body.profile_id).to.equal(profile_id);

            globalState.set("merchantConnectorId", response.body.id);

            cy.task("setGlobalState", globalState.data);
          } else {
            // to be updated
            throw new Error(
              `MCA create call failed with status ${response.status} and message ${response.body.message}`
            );
          }
        });
      }
    );
  }
);
Cypress.Commands.add("mcaRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const connector_name = globalState.get("connectorId");
  const merchant_connector_id = globalState.get("merchantConnectorId");
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/connector_accounts/${merchant_connector_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.connector_name).to.equal(connector_name);
      expect(response.body.id).to.include("mca_").and.to.not.be.empty;
      expect(response.body.status).to.equal("active");

      if (
        merchant_connector_id === undefined ||
        merchant_connector_id === null
      ) {
        globalState.set("merchantConnectorId", response.body.id);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `MCA create call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add(
  "mcaUpdateCall",
  (
    connectorLabel,
    connectorName,
    connectorType,
    globalState,
    mcaUpdateBody,
    paymentMethodsEnabled
  ) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("adminApiKey");
    const base_url = globalState.get("baseUrl");
    const merchant_connector_id = globalState.get("merchantConnectorId");
    const merchant_id = globalState.get("merchantId");
    const profile_id = globalState.get("profileId");
    const url = `${base_url}/v2/connector_accounts/${merchant_connector_id}`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
    };

    // Update request body
    mcaUpdateBody.merchant_id = merchant_id;
    mcaUpdateBody.connector_label = connectorLabel;
    mcaUpdateBody.connector_type = connectorType;
    mcaUpdateBody.payment_methods_enabled = paymentMethodsEnabled;

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
        ...customHeaders,
      },
      body: mcaUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body.connector_name).to.equal(connectorName);
        expect(response.body.id).to.include("mca_").and.to.not.be.empty;
        expect(response.body.status).to.equal("active");
        expect(response.body.profile_id).to.equal(profile_id);
        expect(
          response.body.connector_webhook_details.merchant_secret
        ).to.equal(mcaUpdateBody.connector_webhook_details.merchant_secret);

        if (
          merchant_connector_id === undefined ||
          merchant_connector_id === null
        ) {
          globalState.set("merchantConnectorId", response.body.id);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `MCA create call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);

// API Key API calls
Cypress.Commands.add("apiKeyCreateCall", (apiKeyCreateBody, globalState) => {
  // Define the necessary variables and constant

  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  // We do not want to keep API Key forever,
  // so we set the expiry to tomorrow as new merchant accounts are created with every run
  const expiry = isoTimeTomorrow();
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/api_keys`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  // Update request body
  apiKeyCreateBody.expiration = expiry;

  cy.request({
    method: "POST",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
      ...customHeaders,
    },
    body: apiKeyCreateBody,
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.merchant_id).to.equal(merchant_id);
      expect(response.body.description).to.equal(apiKeyCreateBody.description);

      // API Key assertions are intentionally excluded to avoid being exposed in the logs
      if (base_url.includes("sandbox") || base_url.includes("integ")) {
        expect(response.body).to.have.property("key_id").and.to.include("snd_")
          .and.to.not.be.empty;
      } else if (base_url.includes("localhost")) {
        expect(response.body).to.have.property("key_id").and.to.include("dev_")
          .and.to.not.be.empty;
      }

      globalState.set("apiKeyId", response.body.key_id);
      globalState.set("apiKey", response.body.api_key);

      cy.task("setGlobalState", globalState.data);
    } else {
      // to be updated
      throw new Error(
        `API Key create call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("apiKeyRetrieveCall", (globalState) => {
  // Define the necessary variables and constant
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const api_key_id = globalState.get("apiKeyId");
  const url = `${base_url}/v2/api_keys/${api_key_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.merchant_id).to.equal(merchant_id);

      // API Key assertions are intentionally excluded to avoid being exposed in the logs
      if (base_url.includes("sandbox") || base_url.includes("integ")) {
        expect(response.body).to.have.property("key_id").and.to.include("snd_")
          .and.to.not.be.empty;
      } else if (base_url.includes("localhost")) {
        expect(response.body).to.have.property("key_id").and.to.include("dev_")
          .and.to.not.be.empty;
      }

      if (api_key === undefined || api_key === null) {
        globalState.set("apiKey", response.body.api_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `API Key create call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("apiKeyUpdateCall", (apiKeyUpdateBody, globalState) => {
  // Define the necessary variables and constant
  const api_key = globalState.get("adminApiKey");
  const api_key_id = globalState.get("apiKeyId");
  const base_url = globalState.get("baseUrl");
  // We do not want to keep API Key forever,
  // so we set the expiry to tomorrow as new merchant accounts are created with every run
  const expiry = isoTimeTomorrow();
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/api_keys/${api_key_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  // Update request body
  apiKeyUpdateBody.expiration = expiry;

  cy.request({
    method: "PUT",
    url: url,
    headers: {
      "Content-Type": "application/json",
      "api-key": api_key,
      ...customHeaders,
    },
    body: apiKeyUpdateBody,
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.merchant_id).to.equal(merchant_id);
      expect(response.body.description).to.equal(apiKeyUpdateBody.description);

      // API Key assertions are intentionally excluded to avoid being exposed in the logs
      if (base_url.includes("sandbox") || base_url.includes("integ")) {
        expect(response.body).to.have.property("key_id").and.to.include("snd_")
          .and.to.not.be.empty;
      } else if (base_url.includes("localhost")) {
        expect(response.body).to.have.property("key_id").and.to.include("dev_")
          .and.to.not.be.empty;
      }

      if (api_key === undefined || api_key === null) {
        globalState.set("apiKey", response.body.api_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `API Key create call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});

// Routing API calls
Cypress.Commands.add(
  "routingSetupCall",
  (routingSetupBody, type, payload, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("userInfoToken");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const url = `${base_url}/v2/routing_algorithm`;

    // Update request body
    routingSetupBody.algorithm.data = payload.data;
    routingSetupBody.algorithm.type = type;
    routingSetupBody.description = payload.description;
    routingSetupBody.name = payload.name;
    routingSetupBody.profile_id = profile_id;

    cy.request({
      method: "POST",
      url: url,
      headers: {
        Authorization: `Bearer ${api_key}`,
        "Content-Type": "application/json",
      },
      body: routingSetupBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body).to.have.property("id").and.to.include("routing_");
        expect(response.body).to.have.property("kind").and.to.equal(type);
        expect(response.body)
          .to.have.property("profile_id")
          .and.to.equal(profile_id);

        globalState.set("routingAlgorithmId", response.body.id);
      } else {
        // to be updated
        throw new Error(
          `Routing algorithm setup call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add(
  "routingActivateCall",
  (routingActivationBody, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("userInfoToken");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const routing_algorithm_id = globalState.get("routingAlgorithmId");
    const url = `${base_url}/v2/profiles/${profile_id}/activate_routing_algorithm`;

    // Update request body
    routingActivationBody.routing_algorithm_id = routing_algorithm_id;

    cy.request({
      method: "PATCH",
      url: url,
      headers: {
        Authorization: `Bearer ${api_key}`,
        "Content-Type": "application/json",
      },
      body: routingActivationBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body).to.have.property("id").and.to.include("routing_");
        expect(response.body)
          .to.have.property("profile_id")
          .and.to.equal(profile_id);
      } else {
        // to be updated
        throw new Error(
          `Routing algorithm activation call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add("routingActivationRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("userInfoToken");
  const base_url = globalState.get("baseUrl");
  const profile_id = globalState.get("profileId");
  const query_params = "limit=10";
  const routing_algorithm_id = globalState.get("routingAlgorithmId");
  const url = `${base_url}/v2/profiles/${profile_id}/routing_algorithm?${query_params}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.be.an("array").and.to.not.be.empty;
      for (const key in response.body) {
        expect(response.body[key])
          .to.have.property("id")
          .and.to.include("routing_");
        expect(response.body[key])
          .to.have.property("profile_id")
          .and.to.equal(profile_id);
      }
    } else {
      // to be updated
      throw new Error(
        `Routing algorithm activation retrieve call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("routingDeactivateCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("userInfoToken");
  const base_url = globalState.get("baseUrl");
  const profile_id = globalState.get("profileId");
  const routing_algorithm_id = globalState.get("routingAlgorithmId");
  const url = `${base_url}/v2/profiles/${profile_id}/deactivate_routing_algorithm`;

  cy.request({
    method: "PATCH",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body)
        .to.have.property("id")
        .and.to.include("routing_")
        .and.to.equal(routing_algorithm_id);
      expect(response.body)
        .to.have.property("profile_id")
        .and.to.equal(profile_id);
    } else {
      // to be updated
      throw new Error(
        `Routing algorithm deactivation call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("routingRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("userInfoToken");
  const base_url = globalState.get("baseUrl");
  const profile_id = globalState.get("profileId");
  const routing_algorithm_id = globalState.get("routingAlgorithmId");
  const url = `${base_url}/v2/routing_algorithm/${routing_algorithm_id}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body)
        .to.have.property("id")
        .and.to.include("routing_")
        .and.to.equal(routing_algorithm_id);
      expect(response.body)
        .to.have.property("profile_id")
        .and.to.equal(profile_id);
      expect(response.body).to.have.property("algorithm").and.to.be.a("object")
        .and.not.be.empty;
    } else {
      // to be updated
      throw new Error(
        `Routing algorithm activation retrieve call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add(
  "routingDefaultFallbackCall",
  (routingDefaultFallbackBody, payload, globalState) => {
    // Define the necessary variables and constants
    const api_key = globalState.get("userInfoToken");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const routing_algorithm_id = globalState.get("routingAlgorithmId");
    const url = `${base_url}/v2/profiles/${profile_id}/fallback_routing`;

    // Update request body
    routingDefaultFallbackBody = payload;

    cy.request({
      method: "POST",
      url: url,
      headers: {
        Authorization: `Bearer ${api_key}`,
        "Content-Type": "application/json",
      },
      body: routingDefaultFallbackBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body).to.deep.equal(routingDefaultFallbackBody);
      } else {
        // to be updated
        throw new Error(
          `Routing algorithm activation retrieve call failed with status ${response.status} and message ${response.body.message}`
        );
      }
    });
  }
);
Cypress.Commands.add("routingFallbackRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("userInfoToken");
  const base_url = globalState.get("baseUrl");
  const profile_id = globalState.get("profileId");
  const url = `${base_url}/v2/profiles/${profile_id}/fallback_routing`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.be.an("array").and.to.not.be.empty;
    } else {
      // to be updated
      throw new Error(
        `Routing algorithm activation retrieve call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});

// User API calls
// Below 3 commands should be called in sequence to login a user
Cypress.Commands.add("userLogin", (globalState) => {
  // Define the necessary variables and constant
  const base_url = globalState.get("baseUrl");
  const query_params = `token_only=true`;
  const signin_body = {
    email: `${globalState.get("email")}`,
    password: `${globalState.get("password")}`,
  };
  const url = `${base_url}/user/v2/signin?${query_params}`;

  cy.request({
    method: "POST",
    url: url,
    headers: {
      "Content-Type": "application/json",
    },
    body: signin_body,
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      if (response.body.token_type === "totp") {
        expect(response.body).to.have.property("token").and.to.not.be.empty;

        globalState.set("totpToken", response.body.token);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `User login call failed to get totp token with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("terminate2Fa", (globalState) => {
  // Define the necessary variables and constant
  const base_url = globalState.get("baseUrl");
  const query_params = `skip_two_factor_auth=true`;
  const api_key = globalState.get("totpToken");
  const url = `${base_url}/user/2fa/terminate?${query_params}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      if (response.body.token_type === "user_info") {
        expect(response.body).to.have.property("token").and.to.not.be.empty;

        globalState.set("userInfoToken", response.body.token);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `2FA terminate call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
Cypress.Commands.add("userInfo", (globalState) => {
  // Define the necessary variables and constant
  const base_url = globalState.get("baseUrl");
  const api_key = globalState.get("userInfoToken");
  const url = `${base_url}/user`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `Bearer ${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.have.property("merchant_id").and.to.not.be.empty;
      expect(response.body).to.have.property("org_id").and.to.not.be.empty;
      expect(response.body).to.have.property("profile_id").and.to.not.be.empty;

      globalState.set("merchantId", response.body.merchant_id);
      globalState.set("organizationId", response.body.org_id);
      globalState.set("profileId", response.body.profile_id);
    } else {
      // to be updated
      throw new Error(
        `User login call failed to fetch user info with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});

// List API calls
Cypress.Commands.add("listMcaCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/account/${merchant_id}/connectors`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "api-key": api_key,
      "Content-Type": "application/json",
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      // Control never comes here until the API endpoints are introduced
      console.log("List MCA call successful");
      globalState.set(
        "adyenMerchantConnectorId",
        response.body[2].merchant_connector_id
      );
      globalState.set(
        "bluesnapMerchantConnectorId",
        response.body[0].merchant_connector_id
      );
      globalState.set(
        "stripeMerchantConnectorId",
        response.body[1].merchant_connector_id
      );
    } else if (response.status === 404) {
      expect(response.body.error)
        .to.have.property("message")
        .and.to.equal("Unrecognized request URL");
      expect(response.body.error)
        .to.have.property("type")
        .and.to.equal("invalid_request");

      // hard code MCA values for now
      if (base_url.includes("integ")) {
        globalState.set("adyenMerchantConnectorId", "mca_YOGOW6CdrjudsT9Mvg7w");
        globalState.set(
          "bluesnapMerchantConnectorId",
          "mca_cdKJoouwpmkHqwVJ1bzV"
        );
        globalState.set(
          "stripeMerchantConnectorId",
          "mca_KyxoOnfLXWE1hzPSsl9H"
        );
      }
    } else {
      // to be updated
      throw new Error(
        `MCA list call failed with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});
// templates
Cypress.Commands.add("", () => {
  cy.request({}).then((response) => {});
});
Cypress.Commands.add("", () => {
  cy.request({}).then((response) => {});
});
Cypress.Commands.add("", () => {
  cy.request({}).then((response) => {});
});
