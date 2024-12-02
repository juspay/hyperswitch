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

// commands.js or your custom support file
import { defaultErrorHandler, getValueByKey } from "../e2e/PaymentUtils/Utils";
import * as RequestBodyUtils from "../utils/RequestBodyUtils";
import { handleRedirection } from "./redirectionHandler";

function logRequestId(xRequestId) {
  if (xRequestId) {
    cy.task("cli_log", "x-request-id -> " + xRequestId);
  } else {
    cy.task("cli_log", "x-request-id is not available in the response headers");
  }
}

Cypress.Commands.add(
  "merchantCreateCallTest",
  (merchantCreateBody, globalState) => {
    const randomMerchantId = RequestBodyUtils.generateRandomString();
    RequestBodyUtils.setMerchantId(merchantCreateBody, randomMerchantId);
    globalState.set("merchantId", randomMerchantId);

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/accounts`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: merchantCreateBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      // Handle the response as needed
      globalState.set("profileId", response.body.default_profile);
      globalState.set("publishableKey", response.body.publishable_key);
      globalState.set("merchantDetails", response.body.merchant_details);
    });
  }
);

Cypress.Commands.add("merchantRetrieveCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/accounts/${merchant_id}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.merchant_id).to.equal(merchant_id);
    expect(response.body.payment_response_hash_key).to.not.be.empty;
    expect(response.body.publishable_key).to.not.be.empty;
    expect(response.body.default_profile).to.not.be.empty;
    expect(response.body.organization_id).to.not.be.empty;
    globalState.set("organizationId", response.body.organization_id);

    if (globalState.get("publishableKey") === undefined) {
      globalState.set("publishableKey", response.body.publishable_key);
    }
  });
});

Cypress.Commands.add("merchantDeleteCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  cy.request({
    method: "DELETE",
    url: `${globalState.get("baseUrl")}/accounts/${merchant_id}`,
    headers: {
      Accept: "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.body.merchant_id).to.equal(merchant_id);
    expect(response.body.deleted).to.equal(true);
  });
});

Cypress.Commands.add("merchantListCall", (globalState) => {
  const organization_id = globalState.get("organizationId");

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/accounts/list?organization_id=${organization_id}`,
    headers: {
      Accept: "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    for (const key in response.body) {
      expect(response.body[key]).to.have.property("merchant_id").and.not.empty;
      expect(response.body[key]).to.have.property("organization_id").and.not
        .empty;
      expect(response.body[key]).to.have.property("default_profile").and.not
        .empty;
    }
  });
});

Cypress.Commands.add(
  "merchantUpdateCall",
  (merchantUpdateBody, globalState) => {
    const merchant_id = globalState.get("merchantId");
    const organization_id = globalState.get("organizationId");
    const publishable_key = globalState.get("publishableKey");
    const merchant_details = globalState.get("merchantDetails");

    merchantUpdateBody.merchant_id = merchant_id;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/accounts/${merchant_id}`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: merchantUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      expect(response.body.merchant_id).to.equal(merchant_id);
      expect(response.body.publishable_key).to.equal(publishable_key);
      expect(response.body.organization_id).to.equal(organization_id);
      expect(response.body.merchant_details).to.not.equal(merchant_details);
    });
  }
);

Cypress.Commands.add(
  "createBusinessProfileTest",
  (createBusinessProfile, globalState) => {
    const merchant_id = globalState.get("merchantId");
    const randomProfileName = `profile_${Math.random().toString(36).substring(7)}`;
    createBusinessProfile.profile_name = randomProfileName;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/account/${merchant_id}/business_profile`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: createBusinessProfile,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      globalState.set("profileId", response.body.profile_id);
      if (response.status === 200) {
        expect(response.body.profile_id).to.not.to.be.null;
      } else {
        throw new Error(
          `Business Profile call failed ${response.body.error.message}`
        );
      }
    });
  }
);

Cypress.Commands.add(
  "UpdateBusinessProfileTest",
  (
    updateBusinessProfile,
    is_connector_agnostic_mit_enabled,
    collect_billing_details_from_wallet_connector,
    collect_shipping_details_from_wallet_connector,
    always_collect_billing_details_from_wallet_connector,
    always_collect_shipping_details_from_wallet_connector,
    globalState
  ) => {
    updateBusinessProfile.is_connector_agnostic_mit_enabled =
      is_connector_agnostic_mit_enabled;
    updateBusinessProfile.collect_shipping_details_from_wallet_connector =
      collect_shipping_details_from_wallet_connector;
    updateBusinessProfile.collect_billing_details_from_wallet_connector =
      collect_billing_details_from_wallet_connector;
    updateBusinessProfile.always_collect_billing_details_from_wallet_connector =
      always_collect_billing_details_from_wallet_connector;
    updateBusinessProfile.always_collect_shipping_details_from_wallet_connector =
      always_collect_shipping_details_from_wallet_connector;
    const merchant_id = globalState.get("merchantId");
    const profile_id = globalState.get("profileId");
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/account/${merchant_id}/business_profile/${profile_id}`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: updateBusinessProfile,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      if (response.status === 200) {
        globalState.set(
          "collectBillingDetails",
          response.body.collect_billing_details_from_wallet_connector
        );
        globalState.set(
          "collectShippingDetails",
          response.body.collect_shipping_details_from_wallet_connector
        );
        globalState.set(
          "alwaysCollectBillingDetails",
          response.body.always_collect_billing_details_from_wallet_connector
        );
        globalState.set(
          "alwaysCollectShippingDetails",
          response.body.always_collect_shipping_details_from_wallet_connector
        );
      }
    });
  }
);

Cypress.Commands.add("apiKeyCreateTest", (apiKeyCreateBody, globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/api_keys/${globalState.get("merchantId")}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    body: apiKeyCreateBody,
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    // Handle the response as needed
    globalState.set("apiKey", response.body.api_key);
    globalState.set("apiKeyId", response.body.key_id);
  });
});

Cypress.Commands.add("apiKeyUpdateCall", (apiKeyUpdateBody, globalState) => {
  const merchant_id = globalState.get("merchantId");
  const api_key_id = globalState.get("apiKeyId");

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/api_keys/${merchant_id}/${api_key_id}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    body: apiKeyUpdateBody,
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    // Handle the response as needed
    expect(response.body.name).to.equal("Updated API Key");
    expect(response.body.key_id).to.equal(api_key_id);
    expect(response.body.merchant_id).to.equal(merchant_id);
  });
});

Cypress.Commands.add("apiKeyRetrieveCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  const api_key_id = globalState.get("apiKeyId");

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/api_keys/${merchant_id}/${api_key_id}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.name).to.equal("Updated API Key");
    expect(response.body.key_id).to.equal(api_key_id);
    expect(response.body.merchant_id).to.equal(merchant_id);
  });
});

Cypress.Commands.add("apiKeyListCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  const base_url = globalState.get("baseUrl");
  cy.request({
    method: "GET",
    url: `${base_url}/api_keys/${merchant_id}/list`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.be.an("array").and.not.empty;
    for (const key in response.body) {
      expect(response.body[key]).to.have.property("name").and.not.empty;
      if (base_url.includes("sandbox") || base_url.includes("integ")) {
        expect(response.body[key]).to.have.property("key_id").include("snd_")
          .and.not.empty;
      } else if (base_url.includes("localhost")) {
        expect(response.body[key]).to.have.property("key_id").include("dev_")
          .and.not.empty;
      }
      expect(response.body[key].merchant_id).to.equal(merchant_id);
    }
  });
});

Cypress.Commands.add("apiKeyDeleteCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  const api_key_id = globalState.get("apiKeyId");

  cy.request({
    method: "DELETE",
    url: `${globalState.get("baseUrl")}/api_keys/${merchant_id}/${api_key_id}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.merchant_id).to.equal(merchant_id);
    expect(response.body.key_id).to.equal(api_key_id);
    expect(response.body.revoked).to.equal(true);
  });
});

Cypress.Commands.add(
  "createNamedConnectorCallTest",
  (
    connectorType,
    createConnectorBody,
    payment_methods_enabled,
    globalState,
    connectorName,
    connectorLabel
  ) => {
    const merchantId = globalState.get("merchantId");
    createConnectorBody.connector_type = connectorType;
    createConnectorBody.connector_name = connectorName;
    createConnectorBody.connector_label = connectorLabel;
    createConnectorBody.payment_methods_enabled = payment_methods_enabled;
    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const authDetails = getValueByKey(
          JSON.stringify(jsonContent),
          connectorName
        );
        createConnectorBody.connector_account_details =
          authDetails.connector_account_details;
        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/account/${merchantId}/connectors`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: createConnectorBody,
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(connectorName).to.equal(response.body.connector_name);
            globalState.set(
              "merchantConnectorId",
              response.body.merchant_connector_id
            );
          } else {
            cy.task(
              "cli_log",
              "response status -> " + JSON.stringify(response.status)
            );

            throw new Error(
              `Connector Create Call Failed ${response.body.error.message}`
            );
          }
        });
      }
    );
  }
);

Cypress.Commands.add(
  "createConnectorCallTest",
  (
    connectorType,
    createConnectorBody,
    payment_methods_enabled,
    globalState
  ) => {
    const merchantId = globalState.get("merchantId");
    createConnectorBody.connector_type = connectorType;
    createConnectorBody.profile_id = globalState.get("profileId");
    createConnectorBody.connector_name = globalState.get("connectorId");
    createConnectorBody.payment_methods_enabled = payment_methods_enabled;
    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const authDetails = getValueByKey(
          JSON.stringify(jsonContent),
          globalState.get("connectorId")
        );
        createConnectorBody.connector_account_details =
          authDetails.connector_account_details;

        if (authDetails && authDetails.metadata) {
          createConnectorBody.metadata = {
            ...createConnectorBody.metadata, // Preserve existing metadata fields
            ...authDetails.metadata, // Merge with authDetails.metadata
          };
        }

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/account/${merchantId}/connectors`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: createConnectorBody,
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(globalState.get("connectorId")).to.equal(
              response.body.connector_name
            );
            globalState.set("profileId", response.body.profile_id);
            globalState.set(
              "merchantConnectorId",
              response.body.merchant_connector_id
            );
          } else {
            cy.task(
              "cli_log",
              "response status -> " + JSON.stringify(response.status)
            );

            throw new Error(
              `Connector Create Call Failed ${response.body.error.message}`
            );
          }
        });
      }
    );
  }
);

Cypress.Commands.add(
  "createPayoutConnectorCallTest",
  (connectorType, createConnectorBody, globalState) => {
    const merchantId = globalState.get("merchantId");
    let connectorName = globalState.get("connectorId");
    createConnectorBody.connector_type = connectorType;
    createConnectorBody.connector_name = connectorName;
    createConnectorBody.connector_type = "payout_processor";
    createConnectorBody.profile_id = globalState.get("profileId");

    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        let authDetails = getValueByKey(
          JSON.stringify(jsonContent),
          `${connectorName}_payout`
        );

        // If the connector does not have payout connector creds in creds file, set payoutsExecution to false
        if (authDetails === null) {
          globalState.set("payoutsExecution", false);
          return false;
        } else {
          globalState.set("payoutsExecution", true);
        }

        createConnectorBody.connector_account_details =
          authDetails.connector_account_details;

        if (authDetails && authDetails.metadata) {
          createConnectorBody.metadata = {
            ...createConnectorBody.metadata, // Preserve existing metadata fields
            ...authDetails.metadata, // Merge with authDetails.metadata
          };
        }

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/account/${merchantId}/connectors`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: createConnectorBody,
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(globalState.get("connectorId")).to.equal(
              response.body.connector_name
            );
            globalState.set(
              "merchantConnectorId",
              response.body.merchant_connector_id
            );
          } else {
            cy.task(
              "cli_log",
              "response status -> " + JSON.stringify(response.status)
            );

            throw new Error(
              `Connector Create Call Failed ${response.body.error.message}`
            );
          }
        });
      }
    );
  }
);

Cypress.Commands.add("connectorRetrieveCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  const connector_id = globalState.get("connectorId");
  const merchant_connector_id = globalState.get("merchantConnectorId");

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/account/${merchant_id}/connectors/${merchant_connector_id}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
      "x-merchant-id": merchant_id,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.connector_name).to.equal(connector_id);
    expect(response.body.merchant_connector_id).to.equal(merchant_connector_id);
  });
});

Cypress.Commands.add("connectorDeleteCall", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  const merchant_connector_id = globalState.get("merchantConnectorId");

  cy.request({
    method: "DELETE",
    url: `${globalState.get("baseUrl")}/account/${merchant_id}/connectors/${merchant_connector_id}`,
    headers: {
      Accept: "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.body.merchant_id).to.equal(merchant_id);
    expect(response.body.merchant_connector_id).to.equal(merchant_connector_id);
    expect(response.body.deleted).to.equal(true);
  });
});

Cypress.Commands.add(
  "connectorUpdateCall",
  (connectorType, updateConnectorBody, globalState) => {
    const merchant_id = globalState.get("merchantId");
    const connector_id = globalState.get("connectorId");
    const merchant_connector_id = globalState.get("merchantConnectorId");
    updateConnectorBody.connector_type = connectorType;

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/account/${merchant_id}/connectors/${merchant_connector_id}`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
        "x-merchant-id": merchant_id,
      },
      body: updateConnectorBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      expect(response.body.connector_name).to.equal(connector_id);
      expect(response.body.merchant_connector_id).to.equal(
        merchant_connector_id
      );
      expect(response.body.connector_label).to.equal("updated_connector_label");
    });
  }
);

// Generic function to list all connectors
Cypress.Commands.add("connectorListByMid", (globalState) => {
  const merchant_id = globalState.get("merchantId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/account/${merchant_id}/connectors`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
      "X-Merchant-Id": merchant_id,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.be.an("array").and.not.empty;
    response.body.forEach((item) => {
      expect(item).to.not.have.property("metadata");
      expect(item).to.not.have.property("additional_merchant_data");
      expect(item).to.not.have.property("connector_wallets_details");
    });
  });
});

Cypress.Commands.add(
  "createCustomerCallTest",
  (customerCreateBody, globalState) => {
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/customers`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      body: customerCreateBody,
    }).then((response) => {
      globalState.set("customerId", response.body.customer_id);
      logRequestId(response.headers["x-request-id"]);
      expect(response.body.customer_id, "customer_id").to.not.be.empty;
      expect(customerCreateBody.email, "email").to.equal(response.body.email);
      expect(customerCreateBody.name, "name").to.equal(response.body.name);
      expect(customerCreateBody.phone, "phone").to.equal(response.body.phone);
      expect(customerCreateBody.metadata, "metadata").to.deep.equal(
        response.body.metadata
      );
      expect(customerCreateBody.address, "address").to.deep.equal(
        response.body.address
      );
      expect(
        customerCreateBody.phone_country_code,
        "phone_country_code"
      ).to.equal(response.body.phone_country_code);
    });
  }
);

Cypress.Commands.add("customerListCall", (globalState) => {
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/list`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    for (const key in response.body) {
      expect(response.body[key]).to.not.be.empty;
    }
  });
});

Cypress.Commands.add("customerRetrieveCall", (globalState) => {
  const customer_id = globalState.get("customerId");

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customer_id}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.body.customer_id).to.equal(customer_id).and.not.be.empty;
  });
});

Cypress.Commands.add(
  "customerUpdateCall",
  (customerUpdateBody, globalState) => {
    const customer_id = globalState.get("customerId");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/customers/${customer_id}`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      body: customerUpdateBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.body.customer_id).to.equal(customer_id);
    });
  }
);

Cypress.Commands.add("ephemeralGenerateCall", (globalState) => {
  const customer_id = globalState.get("customerId");
  const merchant_id = globalState.get("merchantId");

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/ephemeral_keys`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: { customer_id: customer_id },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.body.customer_id).to.equal(customer_id);
    expect(response.body.merchant_id).to.equal(merchant_id);
    expect(response.body.id).to.exist.and.not.be.empty;
    expect(response.body.secret).to.exist.and.not.be.empty;
  });
});

Cypress.Commands.add("customerDeleteCall", (globalState) => {
  const customer_id = globalState.get("customerId");

  cy.request({
    method: "DELETE",
    url: `${globalState.get("baseUrl")}/customers/${customer_id}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.body.customer_id).to.equal(customer_id).and.not.be.empty;
    expect(response.body.customer_deleted).to.equal(true);
    expect(response.body.address_deleted).to.equal(true);
    expect(response.body.payment_methods_deleted).to.equal(true);
  });
});

Cypress.Commands.add(
  "paymentMethodListTestLessThanEqualToOnePaymentMethod",
  (res_data, globalState) => {
    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/account/payment_methods?client_secret=${globalState.get("clientSecret")}`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        expect(response.body).to.have.property("currency");
        if (res_data["payment_methods"].length == 1) {
          function getPaymentMethodType(obj) {
            return obj["payment_methods"][0]["payment_method_types"][0][
              "payment_method_type"
            ];
          }
          expect(getPaymentMethodType(res_data)).to.equal(
            getPaymentMethodType(response.body)
          );
        } else {
          expect(0).to.equal(response.body["payment_methods"].length);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit",
  (res_data, globalState) => {
    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/account/payment_methods?client_secret=${globalState.get("clientSecret")}`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        expect(response.body).to.have.property("currency");
        if (res_data["payment_methods"].length > 0) {
          function getPaymentMethodType(obj) {
            return obj["payment_methods"][0]["payment_method_types"][0][
              "card_networks"
            ][0]["eligible_connectors"]
              .slice()
              .sort();
          }
          let config_payment_method_type = getPaymentMethodType(res_data);
          let response_payment_method_type = getPaymentMethodType(
            response.body
          );
          for (let i = 0; i < response_payment_method_type.length; i++) {
            expect(config_payment_method_type[i]).to.equal(
              response_payment_method_type[i]
            );
          }
        } else {
          expect(0).to.equal(response.body["payment_methods"].length);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "sessionTokenCall",
  (sessionTokenBody, res_data, globalState) => {
    sessionTokenBody.payment_id = globalState.get("paymentID");
    sessionTokenBody.client_secret = globalState.get("clientSecret");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/session_tokens`,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      body: sessionTokenBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        console.log("Actual Response:", response.body.session_token);

        const expectedTokens = res_data.body.session_token;
        const actualTokens = response.body.session_token;

        // Verifying length of array
        expect(actualTokens.length, "arrayLength").to.equal(
          expectedTokens.length
        );

        // Verify specific fields in each session_token object
        expectedTokens.forEach((expectedToken, index) => {
          const actualToken = actualTokens[index];

          // Check specific fields only
          expect(actualToken.wallet_name, "wallet_name").to.equal(
            expectedToken.wallet_name
          );
          expect(actualToken.connector, "connector").to.equal(
            expectedToken.connector
          );

          // Additional checks if necessary
          if (expectedToken.session_token_data) {
            expect(
              actualToken.session_token_data.display_name,
              "display_name"
            ).to.equal(expectedToken.session_token_data.display_name);
            expect(
              actualToken.session_token_data.domain_name,
              "domain_name"
            ).to.equal(expectedToken.session_token_data.domain_name);
            expect(
              actualToken.session_token_data.merchant_identifier,
              "merchant_identifier"
            ).to.equal(expectedToken.session_token_data.merchant_identifier);
            expect(actualToken.session_token_data.psp_id, "psp_id").to.equal(
              expectedToken.session_token_data.psp_id
            );
            expect(
              actualToken.session_token_data.operational_analytics_identifier,
              "operational_analytics_identifier"
            ).to.equal(
              expectedToken.session_token_data.operational_analytics_identifier
            );
            expect(actualToken.session_token_data.signature, "signature").to.not
              .be.empty;
          }
        });
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "createPaymentIntentTest",
  (
    createPaymentBody,
    req_data,
    res_data,
    authentication_type,
    capture_method,
    globalState
  ) => {
    if (
      !createPaymentBody ||
      typeof createPaymentBody !== "object" ||
      !req_data.currency
    ) {
      throw new Error(
        "Invalid parameters provided to createPaymentIntentTest command"
      );
    }

    for (const key in req_data) {
      createPaymentBody[key] = req_data[key];
    }
    createPaymentBody.authentication_type = authentication_type;
    createPaymentBody.capture_method = capture_method;
    createPaymentBody.customer_id = globalState.get("customerId");
    createPaymentBody.profile_id = globalState.get("profileId");
    globalState.set("paymentAmount", createPaymentBody.amount);
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: createPaymentBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");

      if (res_data.status === 200) {
        expect(response.body).to.have.property("client_secret");
        const clientSecret = response.body.client_secret;
        globalState.set("clientSecret", clientSecret);
        globalState.set("paymentID", response.body.payment_id);
        cy.log(clientSecret);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(
            response.body[key],
            `Expected ${res_data.body[key]} but got ${response.body[key]}`
          );
        }
        expect(response.body.payment_id, "payment_id").to.not.be.null;
        expect(response.body.merchant_id, "merchant_id").to.not.be.null;
        expect(createPaymentBody.amount, "amount").to.equal(
          response.body.amount
        );
        expect(createPaymentBody.currency, "currency").to.equal(
          response.body.currency
        );
        expect(createPaymentBody.capture_method, "capture_method").to.equal(
          response.body.capture_method
        );
        expect(
          createPaymentBody.authentication_type,
          "authentication_type"
        ).to.equal(response.body.authentication_type);
        expect(createPaymentBody.description, "description").to.equal(
          response.body.description
        );
        expect(createPaymentBody.email, "email").to.equal(response.body.email);
        expect(createPaymentBody.email, "customer.email").to.equal(
          response.body.customer.email
        );
        expect(createPaymentBody.customer_id, "customer.id").to.equal(
          response.body.customer.id
        );
        expect(createPaymentBody.metadata, "metadata").to.deep.equal(
          response.body.metadata
        );
        expect(
          createPaymentBody.setup_future_usage,
          "setup_future_usage"
        ).to.equal(response.body.setup_future_usage);
        expect(createPaymentBody.amount, "amount_capturable").to.equal(
          response.body.amount_capturable
        );
        expect(response.body.amount_received, "amount_received").to.be.oneOf([
          0,
          null,
        ]);
        expect(response.body.connector, "connector").to.be.null;
        expect(createPaymentBody.capture_method, "capture_method").to.equal(
          response.body.capture_method
        );
        expect(response.body.payment_method, "payment_method").to.be.null;
        expect(response.body.payment_method_data, "payment_method_data").to.be
          .null;
        expect(response.body.merchant_connector_id, "merchant_connector_id").to
          .be.null;
        expect(response.body.payment_method_id, "payment_method_id").to.be.null;
        expect(response.body.payment_method_id, "payment_method_status").to.be
          .null;
        expect(response.body.profile_id, "profile_id").to.not.be.null;
        expect(
          response.body.merchant_order_reference_id,
          "merchant_order_reference_id"
        ).to.be.null;
        expect(response.body.connector_mandate_id, "connector_mandate_id").to.be
          .null;
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add("paymentMethodsCallTest", (globalState) => {
  const clientSecret = globalState.get("clientSecret");
  const paymentIntentID = clientSecret.split("_secret_")[0];

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/account/payment_methods?client_secret=${clientSecret}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("redirect_url");
    expect(response.body).to.have.property("payment_methods");
    if (
      globalState.get("collectBillingDetails") === true ||
      globalState.get("alwaysCollectBillingDetails") === true
    ) {
      expect(
        response.body.collect_billing_details_from_wallets,
        "collectBillingDetailsFromWallets"
      ).to.be.true;
    } else
      expect(
        response.body.collect_billing_details_from_wallets,
        "collectBillingDetailsFromWallets"
      ).to.be.false;

    if (
      globalState.get("collectShippingDetails") === true ||
      globalState.get("alwaysCollectShippingDetails") === true
    ) {
      expect(
        response.body.collect_shipping_details_from_wallets,
        "collectShippingDetailsFromWallets"
      ).to.be.true;
    } else
      expect(
        response.body.collect_shipping_details_from_wallets,
        "collectShippingDetailsFromWallets"
      ).to.be.false;
    globalState.set("paymentID", paymentIntentID);
    cy.log(response);
  });
});

Cypress.Commands.add(
  "createPaymentMethodTest",
  (globalState, req_data, res_data) => {
    req_data.customer_id = globalState.get("customerId");
    const merchant_id = globalState.get("merchantId");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payment_methods`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("apiKey"),
      },
      body: req_data,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        expect(response.body.client_secret, "client_secret").to.include(
          "_secret_"
        ).and.to.not.be.null;
        expect(response.body.payment_method_id, "payment_method_id").to.not.be
          .null;
        expect(response.body.merchant_id, "merchant_id").to.equal(merchant_id);
        expect(req_data.payment_method_type, "payment_method_type").to.equal(
          response.body.payment_method_type
        );
        expect(req_data.payment_method, "payment_method").to.equal(
          response.body.payment_method
        );
        expect(response.body.last_used_at, "last_used_at").to.not.be.null;
        expect(req_data.customer_id, "customer_id").to.equal(
          response.body.customer_id
        );
        globalState.set("paymentMethodId", response.body.payment_method_id);
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add("deletePaymentMethodTest", (globalState, res_data) => {
  const payment_method_id = globalState.get("paymentMethodId");
  cy.request({
    method: "DELETE",
    url: `${globalState.get("baseUrl")}/payment_methods/${payment_method_id}`,
    headers: {
      Accept: "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");

    if (response.status === 200) {
      expect(response.body.payment_method_id).to.equal(payment_method_id);
      expect(response.body.deleted).to.be.true;
    } else {
      defaultErrorHandler(response, res_data);
    }
  });
});

Cypress.Commands.add("setDefaultPaymentMethodTest", (globalState) => {
  const payment_method_id = globalState.get("paymentMethodId");
  const customer_id = globalState.get("customerId");
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/customers/${customer_id}/payment_methods/${payment_method_id}/default`,
    headers: {
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    if (response.status === 200) {
      expect(response.body).to.have.property(
        "default_payment_method_id",
        payment_method_id
      );
      expect(response.body).to.have.property("customer_id", customer_id);
    } else {
      defaultErrorHandler(response);
    }
  });
});

Cypress.Commands.add(
  "confirmCallTest",
  (confirmBody, req_data, res_data, confirm, globalState) => {
    const paymentIntentID = globalState.get("paymentID");
    confirmBody.confirm = confirm;
    confirmBody.client_secret = globalState.get("clientSecret");
    for (const key in req_data) {
      confirmBody[key] = req_data[key];
    }
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
      body: confirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", paymentIntentID);
        globalState.set("connectorId", response.body.connector);
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(paymentIntentID, "payment_id").to.equal(
          response.body.payment_id
        );
        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(globalState.get("merchantConnectorId"), "connector_id").to.equal(
          response.body.merchant_connector_id
        );
        expect(response.body.customer, "customer").to.not.be.empty;
        expect(response.body.billing, "billing_address").to.not.be.empty;
        expect(response.body.profile_id, "profile_id").to.not.be.null;

        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "confirmBankRedirectCallTest",
  (confirmBody, req_data, res_data, confirm, globalState) => {
    const paymentIntentId = globalState.get("paymentID");
    const connectorId = globalState.get("connectorId");
    for (const key in req_data) {
      confirmBody[key] = req_data[key];
    }
    confirmBody.confirm = confirm;
    confirmBody.client_secret = globalState.get("clientSecret");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${paymentIntentId}/confirm`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
      body: confirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      if (response.status === 200) {
        expect(response.headers["content-type"]).to.include("application/json");
        globalState.set("paymentID", paymentIntentId);
        globalState.set("connectorId", response.body.connector);
        globalState.set("paymentMethodType", confirmBody.payment_method_type);

        switch (response.body.authentication_type) {
          case "three_ds":
            if (
              response.body.capture_method === "automatic" ||
              response.body.capture_method === "manual"
            ) {
              if (response.body.status !== "failed") {
                // we get many statuses here, hence this verification
                if (
                  connectorId === "adyen" &&
                  response.body.payment_method_type === "blik"
                ) {
                  expect(response.body)
                    .to.have.property("next_action")
                    .to.have.property("type")
                    .to.equal("wait_screen_information");
                } else {
                  expect(response.body)
                    .to.have.property("next_action")
                    .to.have.property("redirect_to_url");
                  globalState.set(
                    "nextActionUrl",
                    response.body.next_action.redirect_to_url
                  );
                }
              } else if (response.body.status === "failed") {
                expect(response.body.error_code).to.equal(
                  res_data.body.error_code
                );
              }
            } else {
              throw new Error(
                `Invalid capture method ${response.body.capture_method}`
              );
            }
            break;
          case "no_three_ds":
            if (
              response.body.capture_method === "automatic" ||
              response.body.capture_method === "manual"
            ) {
              expect(response.body)
                .to.have.property("next_action")
                .to.have.property("redirect_to_url");
              globalState.set(
                "nextActionUrl",
                response.body.next_action.redirect_to_url
              );
            } else {
              throw new Error(
                `Invalid capture method ${response.body.capture_method}`
              );
            }
            break;
          default:
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "confirmBankTransferCallTest",
  (confirmBody, req_data, res_data, confirm, globalState) => {
    const paymentIntentID = globalState.get("paymentID");
    for (const key in req_data) {
      confirmBody[key] = req_data[key];
    }
    confirmBody.confirm = confirm;
    confirmBody.client_secret = globalState.get("clientSecret");
    globalState.set("paymentMethodType", confirmBody.payment_method_type);

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
      body: confirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", paymentIntentID);
        if (
          response.body.capture_method === "automatic" ||
          response.body.capture_method === "manual"
        ) {
          switch (response.body.payment_method_type) {
            case "pix":
              expect(response.body)
                .to.have.property("next_action")
                .to.have.property("qr_code_url");
              if (response.body.next_action.qr_code_url !== null) {
                globalState.set(
                  "nextActionUrl", // This is intentionally kept as nextActionUrl to avoid issues during handleRedirection call,
                  response.body.next_action.qr_code_url
                );
                globalState.set("nextActionType", "qr_code_url");
              } else {
                globalState.set(
                  "nextActionUrl", // This is intentionally kept as nextActionUrl to avoid issues during handleRedirection call,
                  response.body.next_action.image_data_url
                );
                globalState.set("nextActionType", "image_data_url");
              }
              break;
            default:
              expect(response.body)
                .to.have.property("next_action")
                .to.have.property("redirect_to_url");
              globalState.set(
                "nextActionUrl",
                response.body.next_action.redirect_to_url
              );
              break;
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "confirmUpiCall",
  (confirmBody, req_data, res_data, confirm, globalState) => {
    const paymentId = globalState.get("paymentID");
    for (const key in req_data) {
      confirmBody[key] = req_data[key];
    }
    confirmBody.confirm = confirm;
    confirmBody.client_secret = globalState.get("clientSecret");
    globalState.set("paymentMethodType", confirmBody.payment_method_type);

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${paymentId}/confirm`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
      body: confirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        if (
          response.body.capture_method === "automatic" ||
          response.body.capture_method === "manual"
        ) {
          if (response.body.payment_method_type === "upi_collect") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
          } else if (response.body.payment_method_type === "upi_intent") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("qr_code_fetch_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.qr_code_fetch_url
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "createConfirmPaymentTest",
  (
    createConfirmPaymentBody,
    req_data,
    res_data,
    authentication_type,
    capture_method,
    globalState
  ) => {
    createConfirmPaymentBody.authentication_type = authentication_type;
    createConfirmPaymentBody.capture_method = capture_method;
    createConfirmPaymentBody.customer_id = globalState.get("customerId");
    createConfirmPaymentBody.profile_id = globalState.get("profileId");
    for (const key in req_data) {
      createConfirmPaymentBody[key] = req_data[key];
    }
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: createConfirmPaymentBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      globalState.set("clientSecret", response.body.client_secret);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentAmount", createConfirmPaymentBody.amount);
        globalState.set("paymentID", response.body.payment_id);
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(response.body.payment_id, "payment_id").to.equal(
          globalState.get("paymentID")
        );
        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(response.body.merchant_connector_id, "connector_id").to.equal(
          globalState.get("merchantConnectorId")
        );
        expect(response.body.customer, "customer").to.not.be.empty;
        expect(response.body.billing, "billing_address").to.not.be.empty;
        expect(response.body.profile_id, "profile_id").to.not.be.null;
        expect(response.body).to.have.property("status");
        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else {
            throw new Error(
              `Invalid authentication type: ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.deep.equal(
                response.body[key]
              );
            }
          } else {
            throw new Error(
              `Invalid authentication type: ${response.body.authentication_type}`
            );
          }
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

// This is consequent saved card payment confirm call test(Using payment token)
Cypress.Commands.add(
  "saveCardConfirmCallTest",
  (saveCardConfirmBody, req_data, res_data, globalState) => {
    const paymentIntentID = globalState.get("paymentID");
    if (req_data.setup_future_usage === "on_session") {
      saveCardConfirmBody.card_cvc = req_data.payment_method_data.card.card_cvc;
    }
    saveCardConfirmBody.payment_token = globalState.get("paymentToken");
    saveCardConfirmBody.client_secret = globalState.get("clientSecret");
    for (const key in req_data) {
      saveCardConfirmBody[key] = req_data[key];
    }
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("publishableKey"),
      },
      failOnStatusCode: false,
      body: saveCardConfirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", paymentIntentID);

        globalState.set("paymentID", paymentIntentID);
        globalState.set("connectorId", response.body.connector);
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(paymentIntentID, "payment_id").to.equal(
          response.body.payment_id
        );
        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(globalState.get("merchantConnectorId"), "connector_id").to.equal(
          response.body.merchant_connector_id
        );
        expect(response.body.customer, "customer").to.not.be.empty;
        if (req_data.billing !== null) {
          expect(response.body.billing, "billing_address").to.not.be.empty;
        }
        expect(response.body.profile_id, "profile_id").to.not.be.null;
        expect(response.body.payment_token, "payment_token").to.not.be.null;

        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
            expect(response.body.customer_id).to.equal(
              globalState.get("customerId")
            );
          } else {
            // Handle other authentication types as needed
            throw new Error(
              `Invalid authentication type: ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
            expect(response.body.customer_id).to.equal(
              globalState.get("customerId")
            );
          } else {
            // Handle other authentication types as needed
            throw new Error(
              `Invalid authentication type: ${response.body.authentication_type}`
            );
          }
        } else {
          // Handle other capture methods as needed
          throw new Error(
            `Invalid capture method: ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "captureCallTest",
  (requestBody, req_data, res_data, amount_to_capture, globalState) => {
    const payment_id = globalState.get("paymentID");
    requestBody.amount_to_capture = amount_to_capture;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${payment_id}/capture`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");
      if (response.body.capture_method !== undefined) {
        expect(response.body.payment_id).to.equal(payment_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "voidCallTest",
  (requestBody, req_data, res_data, globalState) => {
    const payment_id = globalState.get("paymentID");
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments/${payment_id}/cancel`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "retrievePaymentCallTest",
  (globalState, autoretries = false, attempt = 1) => {
    const payment_id = globalState.get("paymentID");
    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true&expand_attempts=true`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      globalState.set("paymentID", response.body.payment_id);

      expect(response.headers["content-type"]).to.include("application/json");
      expect(response.body.payment_id).to.equal(payment_id);
      expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
      expect(response.body.profile_id, "profile_id").to.not.be.null;
      expect(response.body.billing, "billing_address").to.not.be.empty;
      expect(response.body.customer, "customer").to.not.be.empty;
      if (
        ["succeeded", "processing", "requires_customer_action"].includes(
          response.body.status
        )
      ) {
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(response.body.payment_method, "payment_method").to.not.be.null;
        expect(response.body.merchant_connector_id, "connector_id").to.equal(
          globalState.get("merchantConnectorId")
        );
      }

      if (autoretries) {
        expect(response.body).to.have.property("attempts");
        expect(response.body.attempts).to.be.an("array").and.not.empty;
        expect(response.body.attempts.length).to.equal(attempt);
        expect(response.body.attempts[0].attempt_id).to.include(
          `${payment_id}_`
        );
        for (const key in response.body.attempts) {
          if (
            response.body.attempts[key].attempt_id ===
            `${payment_id}_${attempt}` &&
            response.body.status === "succeeded"
          ) {
            expect(response.body.attempts[key].status).to.equal("charged");
          } else if (
            response.body.attempts[key].attempt_id ===
            `${payment_id}_${attempt}` &&
            response.body.status === "requires_customer_action"
          ) {
            expect(response.body.attempts[key].status).to.equal(
              "authentication_pending"
            );
          } else {
            expect(response.body.attempts[key].status).to.equal("failure");
          }
        }
      }
    });
  }
);

Cypress.Commands.add(
  "refundCallTest",
  (requestBody, req_data, res_data, refund_amount, globalState) => {
    const payment_id = globalState.get("paymentID");
    requestBody.payment_id = payment_id;
    requestBody.amount = refund_amount;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/refunds`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        globalState.set("refundId", response.body.refund_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
        expect(response.body.payment_id).to.equal(payment_id);
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "syncRefundCallTest",
  (req_data, res_data, globalState) => {
    const refundId = globalState.get("refundId");
    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/refunds/${refundId}`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      expect(response.headers["content-type"]).to.include("application/json");
      for (const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
    });
  }
);

Cypress.Commands.add(
  "citForMandatesCallTest",
  (
    requestBody,
    req_data,
    res_data,
    amount,
    confirm,
    capture_method,
    payment_type,
    globalState
  ) => {
    for (const key in req_data) {
      requestBody[key] = req_data[key];
    }
    requestBody.payment_type = payment_type;
    requestBody.confirm = confirm;
    requestBody.amount = amount;
    requestBody.capture_method = capture_method;
    requestBody.customer_id = globalState.get("customerId");
    globalState.set("paymentAmount", requestBody.amount);
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", response.body.payment_id);

        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(globalState.get("merchantConnectorId"), "connector_id").to.equal(
          response.body.merchant_connector_id
        );
        expect(response.body.customer, "customer").to.not.be.empty;
        expect(response.body.profile_id, "profile_id").to.not.be.null;
        if (response.body.status !== "failed") {
          expect(response.body.payment_method_id, "payment_method_id").to.not.be
            .null;
        }

        if (requestBody.mandate_data === null) {
          expect(response.body).to.have.property("payment_method_id");
          globalState.set("paymentMethodId", response.body.payment_method_id);
        } else {
          expect(response.body).to.have.property("mandate_id");
          globalState.set("mandateId", response.body.mandate_id);
        }

        if (response.body.capture_method === "automatic") {
          expect(response.body).to.have.property("mandate_id");
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url
            );
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "mitForMandatesCallTest",
  (
    requestBody,
    req_data,
    res_data,
    amount,
    confirm,
    capture_method,
    globalState
  ) => {
    for (const key in req_data) {
      requestBody[key] = req_data[key];
    }
    requestBody.amount = amount;
    requestBody.confirm = confirm;
    requestBody.capture_method = capture_method;
    requestBody.mandate_id = globalState.get("mandateId");
    requestBody.customer_id = globalState.get("customerId");
    globalState.set("paymentAmount", requestBody.amount);
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", response.body.payment_id);
        expect(response.body.payment_method_data, "payment_method_data").to.not
          .be.empty;
        expect(response.body.connector, "connector").to.equal(
          globalState.get("connectorId")
        );
        expect(globalState.get("merchantConnectorId"), "connector_id").to.equal(
          response.body.merchant_connector_id
        );
        expect(response.body.customer, "customer").to.not.be.empty;
        expect(response.body.profile_id, "profile_id").to.not.be.null;
        expect(response.body.payment_method_id, "payment_method_id").to.not.be
          .null;
        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else if (response.status === 400) {
        if (response.body.error.message === "Mandate Validation Failed") {
          expect(response.body.error.code).to.equal("HE_03");
          expect(response.body.error.message).to.equal(
            "Mandate Validation Failed"
          );
          expect(response.body.error.reason).to.equal(
            "request amount is greater than mandate amount"
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "mitUsingPMId",
  (
    requestBody,
    req_data,
    res_data,
    amount,
    confirm,
    capture_method,
    globalState
  ) => {
    for (const key in req_data) {
      requestBody[key] = req_data[key];
    }
    requestBody.amount = amount;
    requestBody.confirm = confirm;
    requestBody.capture_method = capture_method;
    requestBody.recurring_details.data = globalState.get("paymentMethodId");
    requestBody.customer_id = globalState.get("customerId");
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", response.body.payment_id);
        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "mitUsingNTID",
  (
    requestBody,
    req_data,
    res_data,
    amount,
    confirm,
    capture_method,
    globalState
  ) => {
    for (const key in req_data) {
      requestBody[key] = req_data[key];
    }
    requestBody.amount = amount;
    requestBody.confirm = confirm;
    requestBody.capture_method = capture_method;

    if (globalState.get("connectorId") !== "cybersource") {
      return;
    }

    const apiKey = globalState.get("apiKey");
    const baseUrl = globalState.get("baseUrl");
    const url = `${baseUrl}/payments`;

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": apiKey,
      },
      failOnStatusCode: false,
      body: requestBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.headers["content-type"]).to.include("application/json");

        globalState.set("paymentID", response.body.payment_id);

        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            cy.log(nextActionUrl);
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key], [key]).to.equal(response.body[key]);
            }
          } else {
            throw new Error(
              `Invalid authentication type ${response.body.authentication_type}`
            );
          }
        } else {
          throw new Error(
            `Invalid capture method ${response.body.capture_method}`
          );
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add("listMandateCallTest", (globalState) => {
  const customerId = globalState.get("customerId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customerId}/mandates`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");

    let i = 0;
    for (i in response.body) {
      if (response.body[i].mandate_id === globalState.get("mandateId")) {
        expect(response.body[i].status).to.equal("active");
      }
    }
  });
});

Cypress.Commands.add("revokeMandateCallTest", (globalState) => {
  const mandateId = globalState.get("mandateId");
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/mandates/revoke/${mandateId}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    if (response.body.status === 200) {
      expect(response.body.status).to.equal("revoked");
    } else if (response.body.status === 400) {
      expect(response.body.reason).to.equal("Mandate has already been revoked");
    }
  });
});

Cypress.Commands.add(
  "handleRedirection",
  (globalState, expected_redirection) => {
    let connectorId = globalState.get("connectorId");
    let expected_url = new URL(expected_redirection);
    let redirection_url = new URL(globalState.get("nextActionUrl"));
    handleRedirection(
      "three_ds",
      { redirection_url, expected_url },
      connectorId,
      null
    );
  }
);

Cypress.Commands.add(
  "handleBankRedirectRedirection",
  (globalState, payment_method_type, expected_redirection) => {
    let connectorId = globalState.get("connectorId");
    let expected_url = new URL(expected_redirection);
    let redirection_url = new URL(globalState.get("nextActionUrl"));
    // explicitly restricting `sofort` payment method by adyen from running as it stops other tests from running
    // trying to handle that specific case results in stripe 3ds tests to fail
    if (!(connectorId == "adyen" && payment_method_type == "sofort")) {
      handleRedirection(
        "bank_redirect",
        { redirection_url, expected_url },
        connectorId,
        payment_method_type
      );
    }
  }
);

Cypress.Commands.add(
  "handleBankTransferRedirection",
  (globalState, payment_method_type, expected_redirection) => {
    let connectorId = globalState.get("connectorId");
    let expected_url = new URL(expected_redirection);
    let redirection_url = new URL(globalState.get("nextActionUrl"));
    let next_action_type = globalState.get("nextActionType");
    cy.log(payment_method_type);
    handleRedirection(
      "bank_transfer",
      { redirection_url, expected_url },
      connectorId,
      payment_method_type,
      {
        next_action_type,
      }
    );
  }
);

Cypress.Commands.add(
  "handleUpiRedirection",
  (globalState, payment_method_type, expected_redirection) => {
    let connectorId = globalState.get("connectorId");
    let expected_url = new URL(expected_redirection);
    let redirection_url = new URL(globalState.get("nextActionUrl"));
    handleRedirection(
      "upi",
      { redirection_url, expected_url },
      connectorId,
      payment_method_type
    );
  }
);

Cypress.Commands.add("listCustomerPMCallTest", (globalState) => {
  const customerId = globalState.get("customerId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customerId}/payment_methods`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    if (response.body.customer_payment_methods[0]?.payment_token) {
      const paymentToken =
        response.body.customer_payment_methods[0].payment_token;
      const paymentMethodId =
        response.body.customer_payment_methods[0].payment_method_id;
      globalState.set("paymentToken", paymentToken); // Set paymentToken in globalState
      globalState.set("paymentMethodId", paymentMethodId); // Set paymentMethodId in globalState
    } else {
      // We only get an empty array if something's wrong. One exception is a 4xx when no customer exist but it is handled in the test
      expect(response.body)
        .to.have.property("customer_payment_methods")
        .to.be.an("array").and.empty;
    }
    expect(globalState.get("customerId"), "customer_id").to.equal(
      response.body.customer_payment_methods[0].customer_id
    );
    expect(
      response.body.customer_payment_methods[0].payment_token,
      "payment_token"
    ).to.not.be.null;
    expect(
      response.body.customer_payment_methods[0].payment_method_id,
      "payment_method_id"
    ).to.not.be.null;
    expect(
      response.body.customer_payment_methods[0].payment_method,
      "payment_method"
    ).to.not.be.null;
    expect(
      response.body.customer_payment_methods[0].payment_method_type,
      "payment_method_type"
    ).to.not.be.null;
  });
});

Cypress.Commands.add("listCustomerPMByClientSecret", (globalState) => {
  const clientSecret = globalState.get("clientSecret");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/payment_methods?client_secret=${clientSecret}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    if (response.body.customer_payment_methods[0]?.payment_token) {
      const paymentToken =
        response.body.customer_payment_methods[0].payment_token;
      const paymentMethodId =
        response.body.customer_payment_methods[0].payment_method_id;
      globalState.set("paymentToken", paymentToken);
      globalState.set("paymentMethodId", paymentMethodId);
      expect(
        response.body.customer_payment_methods[0].payment_method_id,
        "payment_method_id"
      ).to.not.be.null;
    } else {
      // We only get an empty array if something's wrong. One exception is a 4xx when no customer exist but it is handled in the test
      expect(response.body)
        .to.have.property("customer_payment_methods")
        .to.be.an("array").and.empty;
    }
  });
});

Cypress.Commands.add("listRefundCallTest", (requestBody, globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/refunds/list`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.data).to.be.an("array").and.not.empty;
  });
});

Cypress.Commands.add(
  "createConfirmPayoutTest",
  (
    createConfirmPayoutBody,
    req_data,
    res_data,
    confirm,
    auto_fulfill,
    globalState
  ) => {
    for (const key in req_data) {
      createConfirmPayoutBody[key] = req_data[key];
    }
    createConfirmPayoutBody.auto_fulfill = auto_fulfill;
    createConfirmPayoutBody.confirm = confirm;
    createConfirmPayoutBody.customer_id = globalState.get("customerId");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payouts/create`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: createConfirmPayoutBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        globalState.set("payoutAmount", createConfirmPayoutBody.amount);
        globalState.set("payoutID", response.body.payout_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "createConfirmWithTokenPayoutTest",
  (
    createConfirmPayoutBody,
    req_data,
    res_data,
    confirm,
    auto_fulfill,
    globalState
  ) => {
    for (const key in req_data) {
      createConfirmPayoutBody[key] = req_data[key];
    }
    createConfirmPayoutBody.customer_id = globalState.get("customerId");
    createConfirmPayoutBody.payout_token = globalState.get("paymentToken");
    createConfirmPayoutBody.auto_fulfill = auto_fulfill;
    createConfirmPayoutBody.confirm = confirm;

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payouts/create`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: createConfirmPayoutBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        globalState.set("payoutAmount", createConfirmPayoutBody.amount);
        globalState.set("payoutID", response.body.payout_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "fulfillPayoutCallTest",
  (payoutFulfillBody, req_data, res_data, globalState) => {
    payoutFulfillBody.payout_id = globalState.get("payoutID");

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payouts/${globalState.get("payoutID")}/fulfill`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: payoutFulfillBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "updatePayoutCallTest",
  (payoutConfirmBody, req_data, res_data, auto_fulfill, globalState) => {
    payoutConfirmBody.confirm = true;
    payoutConfirmBody.auto_fulfill = auto_fulfill;

    cy.request({
      method: "PUT",
      url: `${globalState.get("baseUrl")}/payouts/${globalState.get("payoutID")}`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: payoutConfirmBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add("retrievePayoutCallTest", (globalState) => {
  const payout_id = globalState.get("payoutID");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/payouts/${payout_id}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.payout_id).to.equal(payout_id);
    expect(response.body.amount).to.equal(globalState.get("payoutAmount"));
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
      throw new Error(
        `User login call failed to fetch user info with status ${response.status} and message ${response.body.message}`
      );
    }
  });
});

// Specific to routing tests
Cypress.Commands.add("ListMcaByMid", (globalState) => {
  const merchantId = globalState.get("merchantId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/account/${merchantId}/connectors`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("adminApiKey"),
      "X-Merchant-Id": merchantId,
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    globalState.set("profileId", response.body[0].profile_id);
    globalState.set("stripeMcaId", response.body[0].merchant_connector_id);
    globalState.set("adyenMcaId", response.body[1].merchant_connector_id);
    globalState.set("bluesnapMcaId", response.body[3].merchant_connector_id);
  });
});

Cypress.Commands.add(
  "addRoutingConfig",
  (routingBody, req_data, res_data, type, data, globalState) => {
    for (const key in req_data) {
      routingBody[key] = req_data[key];
    }
    // set profile id from env
    routingBody.profile_id = globalState.get("profileId");
    routingBody.algorithm.type = type;
    routingBody.algorithm.data = data;

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/routing`,
      headers: {
        Authorization: `Bearer ${globalState.get("userInfoToken")}`,
        "Content-Type": "application/json",
        Cookie: `${globalState.get("cookie")}`,
      },
      failOnStatusCode: false,
      body: routingBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        expect(response.body).to.have.property("id");
        globalState.set("routingConfigId", response.body.id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "activateRoutingConfig",
  (req_data, res_data, globalState) => {
    let routing_config_id = globalState.get("routingConfigId");
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/routing/${routing_config_id}/activate`,
      headers: {
        Authorization: `Bearer ${globalState.get("userInfoToken")}`,
        "Content-Type": "application/json",
        Cookie: `${globalState.get("cookie")}`,
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        expect(response.body.id).to.equal(routing_config_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "retrieveRoutingConfig",
  (req_data, res_data, globalState) => {
    let routing_config_id = globalState.get("routingConfigId");
    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/routing/${routing_config_id}`,
      headers: {
        Authorization: `Bearer ${globalState.get("userInfoToken")}`,
        "Content-Type": "application/json",
        Cookie: `${globalState.get("cookie")}`,
      },
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        expect(response.body.id).to.equal(routing_config_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  }
);

Cypress.Commands.add(
  "updateGsmConfig",
  (gsmBody, globalState, step_up_possible) => {
    gsmBody.step_up_possible = step_up_possible;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/gsm/update`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: gsmBody,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);
      if (response.status === 200) {
        expect(response.body)
          .to.have.property("message")
          .to.equal("card_declined");
        expect(response.body).to.have.property("connector").to.equal("stripe");
        expect(response.body)
          .to.have.property("step_up_possible")
          .to.equal(step_up_possible);
      }
    });
  }
);

Cypress.Commands.add(
  "updateConfig",
  (configType, configData, globalState, value) => {
    const base_url = globalState.get("baseUrl");
    const merchant_id = globalState.get("merchantId");
    const api_key = globalState.get("adminApiKey");

    let key;
    let url;
    let body;

    switch (configType) {
      case "autoRetry":
        key = `should_call_gsm_${merchant_id}`;
        url = `${base_url}/configs/${key}`;
        body = { key: key, value: value };
        break;
      case "maxRetries":
        key = `max_auto_retries_enabled_${merchant_id}`;
        url = `${base_url}/configs/${key}`;
        body = { key: key, value: value };
        break;
      case "stepUp":
        key = `step_up_enabled_${merchant_id}`;
        url = `${base_url}/configs/${key}`;
        body = { key: key, value: value };
        break;
      default:
        throw new Error(
          `Invalid config type passed into the configs: "${api_key}: ${value}"`
        );
    }

    cy.request({
      method: "POST",
      url: url,
      headers: {
        "Content-Type": "application/json",
        "api-key": api_key,
      },
      body: body,
      failOnStatusCode: false,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      if (response.status === 200) {
        expect(response.body).to.have.property("key").to.equal(key);
        expect(response.body).to.have.property("value").to.equal(value);
      }
    });
  }
);
