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

import { nanoid } from "nanoid";
import {
  defaultErrorHandler,
  getValueByKey,
} from "../e2e/configs/Payment/Utils.js";
import { isoTimeTomorrow, validateEnv } from "../utils/RequestBodyUtils.js";

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
    const url = `${base_url}/v2/organizations`;

    // Update request body
    organizationCreateBody.organization_name += " " + nanoid();

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        Authorization: `admin-api-key=${api_key}`,
      },
      body: organizationCreateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body)
          .to.have.property("id")
          .and.to.include("org_")
          .and.to.be.a("string").and.not.be.empty;
        globalState.set("organizationId", response.body.id);
        cy.task("setGlobalState", globalState.data);
        expect(response.body).to.have.property("metadata").and.to.equal(null);
      } else {
        // to be updated
        throw new Error(
          `Organization create call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const url = `${base_url}/v2/organizations/${organization_id}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      Authorization: `admin-api-key=${api_key}`,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body)
        .to.have.property("id")
        .and.to.include("org_")
        .and.to.be.a("string").and.not.be.empty;
      expect(response.body.organization_name)
        .to.have.include("Hyperswitch")
        .and.to.be.a("string").and.not.be.empty;

      if (organization_id === undefined || organization_id === null) {
        globalState.set("organizationId", response.body.id);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `Organization retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/organizations/${organization_id}`;

    // Update request body
    organizationUpdateBody.organization_name += " " + nanoid();

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        Authorization: `admin-api-key=${api_key}`,
      },
      body: organizationUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body)
          .to.have.property("id")
          .and.to.include("org_")
          .and.to.be.a("string").and.not.be.empty;
        expect(response.body).to.have.property("metadata").and.to.be.a("object")
          .and.not.be.empty;

        if (organization_id === undefined || organization_id === null) {
          globalState.set("organizationId", response.body.id);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `Organization update call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const key_id_type = "publishable_key";
    const key_id = validateEnv(base_url, key_id_type);
    const organization_id = globalState.get("organizationId");
    const url = `${base_url}/v2/merchant-accounts`;

    const merchant_name = merchantAccountCreateBody.merchant_name
      .replaceAll(" ", "")
      .toLowerCase();

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        Authorization: `admin-api-key=${api_key}`,
        "X-Organization-Id": organization_id,
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

        expect(response.body)
          .to.have.property(key_id_type)
          .and.to.include(key_id).and.to.not.be.empty;

        globalState.set("merchantId", response.body.id);
        globalState.set("publishableKey", response.body.publishable_key);

        cy.task("setGlobalState", globalState.data);
      } else {
        // to be updated
        throw new Error(
          `Merchant create call failed with status ${response.status} and message: "${response.body.error.message}"`
        );
      }
    });
  }
);
Cypress.Commands.add("merchantAccountRetrieveCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const key_id_type = "publishable_key";
  const key_id = validateEnv(base_url, key_id_type);
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/merchant-accounts/${merchant_id}`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      Authorization: `admin-api-key=${api_key}`,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.have.property("id").and.to.be.a("string").and.not
        .be.empty;

      expect(response.body).to.have.property(key_id_type).and.to.include(key_id)
        .and.to.not.be.empty;

      if (merchant_id === undefined || merchant_id === null) {
        globalState.set("merchantId", response.body.id);
        globalState.set("publishableKey", response.body.publishable_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `Merchant account retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const key_id_type = "publishable_key";
    const key_id = validateEnv(base_url, key_id_type);
    const merchant_id = globalState.get("merchantId");
    const url = `${base_url}/v2/merchant-accounts/${merchant_id}`;

    const merchant_name = merchantAccountUpdateBody.merchant_name;

    cy.request({
      method: "PUT",
      url: url,
      headers: {
        "Content-Type": "application/json",
        Authorization: `admin-api-key=${api_key}`,
      },
      body: merchantAccountUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body.id).to.equal(merchant_id);

        expect(response.body)
          .to.have.property(key_id_type)
          .and.to.include(key_id).and.to.not.be.empty;

        expect(response.body.merchant_name).to.equal(merchant_name);

        if (merchant_id === undefined || merchant_id === null) {
          globalState.set("merchantId", response.body.id);
          globalState.set("publishableKey", response.body.publishable_key);
          cy.task("setGlobalState", globalState.data);
        }
      } else {
        // to be updated
        throw new Error(
          `Merchant account update call failed with status ${response.status} and message: "${response.body.error.message}"`
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
        Authorization: `admin-api-key=${api_key}`,
        "x-merchant-id": merchant_id,
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
          `Business profile create call failed with status ${response.status} and message: "${response.body.error.message}"`
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
      Authorization: `admin-api-key=${api_key}`,
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
        `Business profile retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
        Authorization: `admin-api-key=${api_key}`,
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
          `Business profile update call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/connector-accounts`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
      "x-profile-id": profile_id,
    };

    // Update request body
    mcaCreateBody.profile_id = profile_id;
    mcaCreateBody.connector_label = connectorLabel;
    mcaCreateBody.connector_name = connectorName;
    mcaCreateBody.connector_type = connectorType;
    mcaCreateBody.payment_methods_enabled = paymentMethodsEnabled;

    if (connectorName === undefined) {
      throw new Error(
        `Connector name is a mandatory field to create merchant connector account but is undefined.`
      );
    }

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
          mcaCreateBody.metadata = {
            ...mcaCreateBody.metadata, // Preserve existing metadata fields
            ...authDetails.metadata, // Merge with authDetails.metadata
          };
        }

        cy.request({
          method: "POST",
          url: url,
          headers: {
            "Content-Type": "application/json",
            Authorization: `admin-api-key=${api_key}`,
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
              `Merchant connector account create call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const profile_id = globalState.get("profileId");
  const url = `${base_url}/v2/connector-accounts/${merchant_connector_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
    "x-profile-id": profile_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      Authorization: `admin-api-key=${api_key}`,
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
        `Merchant connector account retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/connector-accounts/${merchant_connector_id}`;

    const customHeaders = {
      "x-merchant-id": merchant_id,
    };

    // Update request body
    mcaUpdateBody.merchant_id = merchant_id;
    mcaUpdateBody.connector_label = connectorLabel;
    mcaUpdateBody.connector_type = connectorType;
    mcaUpdateBody.payment_methods_enabled = paymentMethodsEnabled;

    // Read connector auth file and merge metadata
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const jsonString = JSON.stringify(jsonContent);
        const key =
          connectorType === "payment_processor"
            ? connectorName
            : `${connectorName}_payout`;
        const authDetails = getValueByKey(jsonString, key);

        if (authDetails && authDetails.metadata) {
          mcaUpdateBody.metadata = {
            ...mcaUpdateBody.metadata,
            ...authDetails.metadata,
          };
        }

        cy.request({
          method: "PUT",
          url: url,
          headers: {
            "Content-Type": "application/json",
            Authorization: `admin-api-key=${api_key}`,
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
              `Merchant connector account update call failed with status ${response.status} and message: "${response.body.error.message}"`
            );
          }
        });
      }
    );
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
  const key_id_type = "key_id";
  const key_id = validateEnv(base_url, key_id_type);
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/api-keys`;

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
      Authorization: `admin-api-key=${api_key}`,
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
      expect(response.body).to.have.property(key_id_type).and.to.include(key_id)
        .and.to.not.be.empty;

      globalState.set("apiKeyId", response.body.key_id);
      globalState.set("apiKey", response.body.api_key);

      cy.task("setGlobalState", globalState.data);
    } else {
      // to be updated
      throw new Error(
        `API Key create call failed with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});
Cypress.Commands.add("apiKeyRetrieveCall", (globalState) => {
  // Define the necessary variables and constant
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const key_id_type = "key_id";
  const key_id = validateEnv(base_url, key_id_type);
  const merchant_id = globalState.get("merchantId");
  const api_key_id = globalState.get("apiKeyId");
  const url = `${base_url}/v2/api-keys/${api_key_id}`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      "Content-Type": "application/json",
      Authorization: `admin-api-key=${api_key}`,
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body.merchant_id).to.equal(merchant_id);
      // API Key assertions are intentionally excluded to avoid being exposed in the logs
      expect(response.body).to.have.property(key_id_type).and.to.include(key_id)
        .and.to.not.be.empty;

      if (api_key === undefined || api_key === null) {
        globalState.set("apiKey", response.body.api_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `API Key retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const key_id_type = "key_id";
  const key_id = validateEnv(base_url, key_id_type);
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/api-keys/${api_key_id}`;

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
      Authorization: `admin-api-key=${api_key}`,
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
      expect(response.body).to.have.property(key_id_type).and.to.include(key_id)
        .and.to.not.be.empty;

      if (api_key === undefined || api_key === null) {
        globalState.set("apiKey", response.body.api_key);
        cy.task("setGlobalState", globalState.data);
      }
    } else {
      // to be updated
      throw new Error(
        `API Key update call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/routing-algorithms`;

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
          `Routing algorithm setup call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/profiles/${profile_id}/activate-routing-algorithm`;

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
          `Routing algorithm activation call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const url = `${base_url}/v2/profiles/${profile_id}/routing-algorithm?${query_params}`;

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
        `Routing algorithm activation retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const url = `${base_url}/v2/profiles/${profile_id}/deactivate-routing-algorithm`;

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
        `Routing algorithm deactivation call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const url = `${base_url}/v2/routing-algorithms/${routing_algorithm_id}`;

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
        `Routing algorithm activation retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
    const url = `${base_url}/v2/profiles/${profile_id}/fallback-routing`;

    // Update request body
    routingDefaultFallbackBody = payload;

    cy.request({
      method: "PATCH",
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
          `Routing algorithm activation retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
  const url = `${base_url}/v2/profiles/${profile_id}/fallback-routing`;

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
        `Routing algorithm activation retrieve call failed with status ${response.status} and message: "${response.body.error.message}"`
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
        `User login call failed to get totp token with status ${response.status} and message: "${response.body.error.message}"`
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
        `2FA terminate call failed with status ${response.status} and message: "${response.body.error.message}"`
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
        `User login call failed to fetch user info with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});

// List API calls
Cypress.Commands.add("merchantAccountsListCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const key_id_type = "publishable_key";
  const key_id = validateEnv(base_url, key_id_type);
  const organization_id = globalState.get("organizationId");
  const url = `${base_url}/v2/organizations/${organization_id}/merchant-accounts`;

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `admin-api-key=${api_key}`,
      "Content-Type": "application/json",
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.be.an("array").and.to.not.be.empty;
      for (const key in response.body) {
        expect(response.body[key]).to.have.property("id").and.to.not.be.empty;
        expect(response.body[key])
          .to.have.property("organization_id")
          .and.to.equal(organization_id);
        expect(response.body[key])
          .to.have.property(key_id_type)
          .and.include(key_id).and.to.not.be.empty;
        expect(response.body[key]).to.have.property("id").and.to.not.be.empty;
      }
    } else {
      // to be updated
      throw new Error(
        `Merchant accounts list call failed with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});
Cypress.Commands.add("businessProfilesListCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/merchant-accounts/${merchant_id}/profiles`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `admin-api-key=${api_key}`,
      "Content-Type": "application/json",
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      expect(response.body).to.be.an("array").and.to.not.be.empty;
      for (const key in response.body) {
        expect(response.body[key]).to.have.property("id").and.to.not.be.empty;
        expect(response.body[key])
          .to.have.property("merchant_id")
          .and.to.equal(merchant_id);
        expect(response.body[key]).to.have.property("payment_response_hash_key")
          .and.to.not.be.empty;
      }
    } else {
      // to be updated
      throw new Error(
        `Business profiles list call failed with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});
Cypress.Commands.add("mcaListCall", (globalState, service_type) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const merchant_id = globalState.get("merchantId");
  const profile_id = globalState.get("profileId");
  const url = `${base_url}/v2/profiles/${profile_id}/connector-accounts`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
    "x-profile-id": profile_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `admin-api-key=${api_key}`,
      "Content-Type": "application/json",
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      // TODO: Update List MCA such that it should handle cases for both routing as well normal calls
      // TODO: Present implementation looks a bit hacky
      if (service_type === "routing") {
        if (response.body[0].connector_name === "stripe")
          globalState.set("stripeMerchantConnectorId", response.body[0].id);
        if (response.body[1].connector_name === "adyen")
          globalState.set("adyenMerchantConnectorId", response.body[1].id);
        if (response.body[2].connector_name === "bluesnap")
          globalState.set("bluesnapMerchantConnectorId", response.body[2].id);
      } else {
        expect(response.body).to.be.an("array").and.to.not.be.empty;
        for (const key in response.body) {
          expect(response.body[key]).to.have.property("connector_name").and.to
            .not.be.empty;
          expect(response.body[key]).to.have.property("connector_label").and.to
            .not.be.empty;
          expect(response.body[key]).to.have.property("id").and.to.not.be.empty;
          expect(response.body[key])
            .to.have.property("payment_methods_enabled")
            .and.to.be.an("array").and.to.not.be.empty;
          expect(response.body[key])
            .to.have.property("profile_id")
            .and.to.equal(profile_id);
          expect(response.body[key])
            .to.have.property("status")
            .and.to.equal("active");
        }
      }
    } else {
      // to be updated
      throw new Error(
        `Merchant connector account list call failed with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});
Cypress.Commands.add("apiKeysListCall", (globalState) => {
  // Define the necessary variables and constants
  const api_key = globalState.get("adminApiKey");
  const base_url = globalState.get("baseUrl");
  const key_id_type = "key_id";
  const key_id = validateEnv(base_url, key_id_type);
  const merchant_id = globalState.get("merchantId");
  const url = `${base_url}/v2/api-keys/list`;

  const customHeaders = {
    "x-merchant-id": merchant_id,
  };

  cy.request({
    method: "GET",
    url: url,
    headers: {
      Authorization: `admin-api-key=${api_key}`,
      "Content-Type": "application/json",
      ...customHeaders,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    if (response.status === 200) {
      // This end point does not work
      expect(response.body).to.be.an("array").and.to.not.be.empty;
      for (const key in response.body) {
        expect(response.body[key])
          .to.have.property(key_id_type)
          .and.to.include(key_id).and.to.not.be.empty;
        expect(response.body[key])
          .to.have.property("merchant_id")
          .and.to.equal(merchant_id).and.to.not.be.empty;
      }
    } else {
      // to be updated
      throw new Error(
        `API Keys list call failed with status ${response.status} and message: "${response.body.error.message}"`
      );
    }
  });
});

// Payment API calls
// Update the below commands while following the conventions
// Below is an example of how the payment intent create call should look like (update the below command as per the need)

Cypress.Commands.add(
  "paymentVoidCall",
  (globalState, voidRequestBody, data) => {
    const { Request: reqData = {}, Response: resData } = data || {};

    // Define the necessary variables and constants at the top
    const api_key = globalState.get("apiKey");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const payment_id = globalState.get("paymentID");
    const url = `${base_url}/v2/payments/${payment_id}/cancel`;

    // Apply connector-specific request data (including cancellation_reason)
    for (const key in reqData) {
      voidRequestBody[key] = reqData[key];
    }

    // Pass Custom Headers
    const customHeaders = {
      "x-profile-id": profile_id,
    };

    cy.request({
      method: "POST",
      url: url,
      headers: {
        Authorization: `api-key=${api_key}`,
        "Content-Type": "application/json",
        ...customHeaders,
      },
      body: voidRequestBody,
      failOnStatusCode: false,
    }).then((response) => {
      // Logging x-request-id is mandatory
      logRequestId(response.headers["x-request-id"]);

      cy.wrap(response).then(() => {
        expect(response.headers["content-type"]).to.include("application/json");
        if (response.status === 200) {
          for (const key in resData.body) {
            expect(resData.body[key]).to.equal(response.body[key]);
          }
        } else {
          defaultErrorHandler(response, resData);
        }
      });
    });
  }
);
Cypress.Commands.add(
  "paymentIntentCreateCall",
  (
    globalState,
    paymentRequestBody,
    paymentResponseBody,
    authentication_type,
    capture_method
  ) => {
    // Define the necessary variables and constants at the top
    // Also construct the URL here
    const api_key = globalState.get("apiKey");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const url = `${base_url}/v2/payments/create-intent`;

    // Set capture_method and authentication_type as parameters (like V1)
    paymentRequestBody.authentication_type = authentication_type;
    paymentRequestBody.capture_method = capture_method;

    // Pass Custom Headers
    const customHeaders = {
      "x-profile-id": profile_id,
    };

    cy.request({
      method: "POST",
      url: url,
      headers: {
        Authorization: `api-key=${api_key}`,
        "Content-Type": "application/json",
        ...customHeaders,
      },
      body: paymentRequestBody,
      failOnStatusCode: false,
    }).then((response) => {
      // Logging x-request-id is mandatory
      logRequestId(response.headers["x-request-id"]);

      cy.wrap(response).then(() => {
        expect(response.headers["content-type"]).to.include("application/json");
        if (response.status === 200) {
          // Validate the payment create response - V2 uses different ID format
          expect(response.body).to.have.property("id").and.to.be.a("string").and
            .not.be.empty;
          expect(response.body).to.have.property("status");

          // Store the payment ID for future use
          globalState.set("paymentID", response.body.id);

          // Log payment creation success
          cy.task(
            "cli_log",
            `Payment created with ID: ${response.body.id}, Status: ${response.body.status}`
          );

          if (paymentResponseBody && paymentResponseBody.body) {
            for (const key in paymentResponseBody.body) {
              if (paymentResponseBody.body[key] !== null) {
                expect(response.body[key]).to.equal(
                  paymentResponseBody.body[key]
                );
              }
            }
          }
        } else {
          defaultErrorHandler(response, paymentResponseBody);
        }
      });
    });
  }
);
Cypress.Commands.add(
  "paymentConfirmCall",
  (globalState, paymentConfirmRequestBody, data) => {
    const { Request: reqData = {}, Response: resData } = data || {};

    // Define the necessary variables and constants at the top
    const api_key = globalState.get("apiKey");
    const base_url = globalState.get("baseUrl");
    const profile_id = globalState.get("profileId");
    const payment_id = globalState.get("paymentID");
    const url = `${base_url}/v2/payments/${payment_id}/confirm-intent`;

    // Apply connector-specific request data
    for (const key in reqData) {
      paymentConfirmRequestBody[key] = reqData[key];
    }

    // Pass Custom Headers
    const customHeaders = {
      "x-profile-id": profile_id,
    };

    cy.request({
      method: "POST",
      url: url,
      headers: {
        Authorization: `api-key=${api_key}`,
        "Content-Type": "application/json",
        ...customHeaders,
      },
      body: paymentConfirmRequestBody,
      failOnStatusCode: false,
    }).then((response) => {
      // Logging x-request-id is mandatory
      logRequestId(response.headers["x-request-id"]);

      cy.wrap(response).then(() => {
        expect(response.headers["content-type"]).to.include("application/json");
        if (response.status === 200) {
          // Validate the payment confirm response
          expect(response.body).to.have.property("id").and.to.be.a("string").and
            .not.be.empty;
          expect(response.body).to.have.property("status");

          globalState.set("paymentID", response.body.id);

          // Validate response body against expected data
          if (resData && resData.body) {
            for (const key in resData.body) {
              // Skip validation if expected value is null or undefined
              if (resData.body[key] == null) {
                continue;
              }
              // Only validate if the field exists in the response and has a non-null value
              if (
                response.body.hasOwnProperty(key) &&
                response.body[key] != null
              ) {
                // Use deep equal for object comparison, regular equal for primitives
                if (
                  typeof resData.body[key] === "object" &&
                  typeof response.body[key] === "object"
                ) {
                  expect(response.body[key]).to.deep.equal(resData.body[key]);
                } else {
                  expect(response.body[key]).to.equal(resData.body[key]);
                }
              }
            }
          }
        } else {
          defaultErrorHandler(response, resData);
        }
      });
    });
  }
);
