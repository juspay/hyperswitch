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
import * as RequestBodyUtils from "../utils/RequestBodyUtils";

function logRequestId(xRequestId) {
  if (xRequestId) {
    cy.task('cli_log', "x-request-id -> " + xRequestId);
  } else {
    cy.task('cli_log', "x-request-id is not available in the response headers");
  }
}

Cypress.Commands.add("merchantCreateCallTest", (merchantCreateBody, globalState) => {
  const randomMerchantId = RequestBodyUtils.generateRandomString();
  RequestBodyUtils.setMerchantId(merchantCreateBody, randomMerchantId);
  globalState.set("merchantId", randomMerchantId);

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/accounts`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    body: merchantCreateBody,
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    // Handle the response as needed
    globalState.set("publishableKey", response.body.publishable_key);
  });
});

Cypress.Commands.add("apiKeyCreateTest", (apiKeyCreateBody, globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/api_keys/${globalState.get("merchantId")}`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": globalState.get("adminApiKey"),
    },
    body: apiKeyCreateBody,
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    // Handle the response as needed
    globalState.set("apiKey", response.body.api_key);
  });
});

Cypress.Commands.add("createConnectorCallTest", (createConnectorBody, globalState) => {
  const merchantId = globalState.get("merchantId");
  createConnectorBody.connector_name = globalState.get("connectorId");
  // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
  // it is best to use then() to handle the response within the same block of code
  cy.readFile(globalState.get("connectorAuthFilePath")).then((jsonContent) => {
    const authDetails = getValueByKey(JSON.stringify(jsonContent), globalState.get("connectorId"));
    createConnectorBody.connector_account_details = authDetails;
    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/account/${merchantId}/connectors`,
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: createConnectorBody,
      failOnStatusCode: false
    }).then((response) => {
      logRequestId(response.headers['x-request-id']);

      if (response.status === 200) {
        expect(globalState.get("connectorId")).to.equal(response.body.connector_name);
      } else {
        cy.task('cli_log', "response status -> " + JSON.stringify(response.status));
      }
    });
  });
});

function getValueByKey(jsonObject, key) {
  const data = typeof jsonObject === 'string' ? JSON.parse(jsonObject) : jsonObject;
  if (data && typeof data === 'object' && key in data) {
    return data[key];
  } else {
    return null;
  }
}

Cypress.Commands.add("createCustomerCallTest", (customerCreateBody, globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/customers`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: customerCreateBody,
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    globalState.set("customerId", response.body.customer_id);
  });
});

Cypress.Commands.add("createPaymentIntentTest", (request, req_data, res_data, authentication_type, capture_method, globalState) => {
  if (!request || typeof request !== "object" || !req_data.currency || !authentication_type) {
    throw new Error("Invalid parameters provided to createPaymentIntentTest command");
  }
  request.currency = req_data.currency;
  request.authentication_type = authentication_type;
  request.capture_method = capture_method;
  request.setup_future_usage = req_data.setup_future_usage;
  request.customer_acceptance = req_data.customer_acceptance;
  request.customer_id = globalState.get("customerId");
  globalState.set("paymentAmount", request.amount);
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
    body: request,
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    
    if(response.status === 200){
      expect(response.body).to.have.property("client_secret");
      const clientSecret = response.body.client_secret;
      globalState.set("clientSecret", clientSecret);
      globalState.set("paymentID", response.body.payment_id);
      cy.log(clientSecret);
      for(const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
      expect(request.amount).to.equal(response.body.amount);
      expect(null).to.equal(response.body.amount_received);
      expect(request.amount).to.equal(response.body.amount_capturable);
    }
    else {
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
  });
});

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
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("redirect_url");
    expect(response.body).to.have.property("payment_methods");
    globalState.set("paymentID", paymentIntentID);
    cy.log(response);
  });
});

Cypress.Commands.add("confirmCallTest", (confirmBody, req_data, res_data, confirm, globalState) => {
  const paymentIntentID = globalState.get("paymentID");
  confirmBody.payment_method_data.card = req_data.card;
  confirmBody.confirm = confirm;
  confirmBody.client_secret = globalState.get("clientSecret");
  confirmBody.customer_acceptance = req_data.customer_acceptance;

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
    logRequestId(response.headers['x-request-id']);

    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    if(response.status === 200){
      globalState.set("paymentID", paymentIntentID);
      if (response.body.capture_method === "automatic") {
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
            .to.have.property("redirect_to_url");
          globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
        } else if (response.body.authentication_type === "no_three_ds") {
          for(const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }
        } else {
          // Handle other authentication types as needed
          throw new Error(`Unsupported authentication type: ${authentication_type}`);
        }
      } else if (response.body.capture_method === "manual") {
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
            .to.have.property("redirect_to_url")
          globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
        }
        else if (response.body.authentication_type === "no_three_ds") {
          for(const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }      } else {
          // Handle other authentication types as needed
          throw new Error(`Unsupported authentication type: ${authentication_type}`);
        }
      }
    }
    else {
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
    
  });
});

Cypress.Commands.add("createConfirmPaymentTest", (createConfirmPaymentBody, req_data, res_data, authentication_type, capture_method, globalState) => {
  createConfirmPaymentBody.payment_method_data.card = req_data.card;
  createConfirmPaymentBody.authentication_type = authentication_type;
  createConfirmPaymentBody.currency = req_data.currency;
  createConfirmPaymentBody.capture_method = capture_method;
  createConfirmPaymentBody.customer_acceptance = req_data.customer_acceptance;
  createConfirmPaymentBody.setup_future_usage = req_data.setup_future_usage;
  createConfirmPaymentBody.customer_id = globalState.get("customerId");

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
    logRequestId(response.headers['x-request-id']);

    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");

    if(response.status === 200){
      if (response.body.capture_method === "automatic") {
        expect(response.body).to.have.property("status");
        globalState.set("paymentAmount", createConfirmPaymentBody.amount);
        globalState.set("paymentID", response.body.payment_id);
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
            .to.have.property("redirect_to_url")
            globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
        }
        else if (response.body.authentication_type === "no_three_ds") {
          for(const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }
        } else {
          // Handle other authentication types as needed
          throw new Error(`Unsupported authentication type: ${authentication_type}`);
        }
      }
      else if (response.body.capture_method === "manual") {
        expect(response.body).to.have.property("status");
        globalState.set("paymentAmount", createConfirmPaymentBody.amount);
        globalState.set("paymentID", response.body.payment_id);
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
            .to.have.property("redirect_to_url")
            globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
        }
        else if (response.body.authentication_type === "no_three_ds") {
          for(const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }      } else {
          // Handle other authentication types as needed
          throw new Error(`Unsupported authentication type: ${authentication_type}`);
        }
      }
    }
    else{
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
  });
});

// This is consequent saved card payment confirm call test(Using payment token)
Cypress.Commands.add("saveCardConfirmCallTest", (SaveCardConfirmBody, req_data, res_data,globalState) => {
  const paymentIntentID = globalState.get("paymentID");
  SaveCardConfirmBody.card_cvc = req_data.card.card_cvc;
  SaveCardConfirmBody.payment_token = globalState.get("paymentToken");
  SaveCardConfirmBody.client_secret = globalState.get("clientSecret");
  console.log("conf conn ->" + globalState.get("connectorId"));
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
    failOnStatusCode: false,
    body: SaveCardConfirmBody,
  })
    .then((response) => {
      logRequestId(response.headers['x-request-id']);

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      globalState.set("paymentID", paymentIntentID);
      if(response.status === 200){
        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body).to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
          } else if (response.body.authentication_type === "no_three_ds") {
            for(const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
            expect(response.body.customer_id).to.equal(globalState.get("customerId"));
          } else {
            // Handle other authentication types as needed
            throw new Error(`Unsupported authentication type: ${authentication_type}`);
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body).to.have.property("next_action")
              .to.have.property("redirect_to_url")
          }
          else if (response.body.authentication_type === "no_three_ds") {
            for(const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }          expect(response.body.customer_id).to.equal(globalState.get("customerId"));
          } else {
            // Handle other authentication types as needed
            throw new Error(`Unsupported authentication type: ${authentication_type}`);
          }
        }
      }
      else {
        expect(response.body).to.have.property("error");
        for(const key in res_data.body.error) {
          expect(res_data.body.error[key]).to.equal(response.body.error[key]);
        }
      }
    });
});

Cypress.Commands.add("captureCallTest", (requestBody, req_data, res_data, amount_to_capture, globalState) => {
  const payment_id = globalState.get("paymentID");
  requestBody.amount_to_capture = amount_to_capture;
  let amount = globalState.get("paymentAmount");
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
    logRequestId(response.headers['x-request-id']);

    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    if(response.body.capture_method !== undefined) {
      expect(response.body.payment_id).to.equal(payment_id);
      for(const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
    }
    else{
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
    
  });
});

Cypress.Commands.add("voidCallTest", (requestBody, req_data, res_data, globalState) => {
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
    logRequestId(response.headers['x-request-id']);

    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    if(response.status === 200) {
      for(const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
    }
    else{
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
  });
});

Cypress.Commands.add("retrievePaymentCallTest", (globalState) => {
  console.log("syncpaymentID ->" + globalState.get("paymentID"));
  const payment_id = globalState.get("paymentID");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.payment_id).to.equal(payment_id);
    expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
    globalState.set("paymentID", response.body.payment_id);

  });
});

Cypress.Commands.add("refundCallTest", (requestBody, req_data, res_data, refund_amount, globalState) => {
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
    body: requestBody
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);
    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    
    if(response.status === 200) {
      globalState.set("refundId", response.body.refund_id);
      for(const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
      expect(response.body.payment_id).to.equal(payment_id);
    }
    else{
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
    
  });
});

Cypress.Commands.add("syncRefundCallTest", (req_data, res_data, globalState) => {
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
    logRequestId(response.headers['x-request-id']);
    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    for(const key in res_data.body) {
      expect(res_data.body[key]).to.equal(response.body[key]);
    }
  });
});

Cypress.Commands.add("citForMandatesCallTest", (requestBody, req_data, res_data, amount, confirm, capture_method, payment_type, globalState) => {
  requestBody.payment_method_data.card = req_data.card;
  requestBody.payment_type = payment_type;
  requestBody.confirm = confirm;
  requestBody.amount = amount;
  requestBody.currency = req_data.currency;
  requestBody.capture_method = capture_method;
  requestBody.mandate_data.mandate_type = req_data.mandate_type;
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
    logRequestId(response.headers['x-request-id']);
    expect(res_data.status).to.equal(response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    globalState.set("mandateId", response.body.mandate_id);
    globalState.set("paymentID", response.body.payment_id);

    if(response.status === 200) {
      if (response.body.capture_method === "automatic") {
        expect(response.body).to.have.property("mandate_id");
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
            .to.have.property("redirect_to_url");
          const nextActionUrl = response.body.next_action.redirect_to_url;
          globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
          cy.log(response.body);
          cy.log(nextActionUrl);
        } else if (response.body.authentication_type === "no_three_ds") {
          for(const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }
        } 
        for(const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      }
      else if (response.body.capture_method === "manual") {
        expect(response.body).to.have.property("mandate_id");
        if (response.body.authentication_type === "three_ds") {
          expect(response.body).to.have.property("next_action")
        }
        for(const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      }
    }
    else{
      expect(response.body).to.have.property("error");
      for(const key in res_data.body.error) {
        expect(res_data.body.error[key]).to.equal(response.body.error[key]);
      }
    }
  });
});

Cypress.Commands.add("mitForMandatesCallTest", (requestBody, amount, confirm, capture_method, globalState) => {
  requestBody.amount = amount;
  requestBody.confirm = confirm;
  requestBody.capture_method = capture_method;
  requestBody.mandate_id = globalState.get("mandateId");
  requestBody.customer_id = globalState.get("customerId");
  globalState.set("paymentAmount", requestBody.amount);
  console.log("mit body " + JSON.stringify(requestBody));
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
    logRequestId(response.headers['x-request-id']);
    expect(response.headers["content-type"]).to.include("application/json");
    globalState.set("paymentID", response.body.payment_id);
    console.log("mit statusss-> " + response.body.status);
    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
        cy.log(response.body);
        cy.log(nextActionUrl);
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal("succeeded");
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else if (response.body.capture_method === "manual") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
        cy.log(response.body);
        cy.log(nextActionUrl);
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal("requires_capture");
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
  });
});


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
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");

    let i = 0;
    for (i in response.body) {
      if (response.body[i].mandate_id === globalState.get("mandateId")) {
        expect(response.body[i].status).to.equal("active");
      }
    };
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
    failOnStatusCode: false
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");
    if (response.body.status === 200) {
      expect(response.body.status).to.equal("revoked");
    } else if (response.body.status === 400) {
      expect(response.body.reason).to.equal("Mandate has already been revoked");
    }
  });
});

Cypress.Commands.add("handleRedirection", (globalState, expected_redirection) => {
  let connectorId = globalState.get("connectorId");
  let expected_url = new URL(expected_redirection);
  let redirection_url = new URL(globalState.get("nextActionUrl"));
  cy.visit(redirection_url.href);
  if (globalState.get("connectorId") == "adyen") {
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('input[type="password"]').click();
        cy.get('input[type="password"]').type("password");
        cy.get('#buttonSubmit').click();
      })
  }
  else if (globalState.get("connectorId") === "cybersource" || globalState.get("connectorId") === "bankofamerica") {
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('input[type="text"]').click().type("1234");
        cy.get('input[value="SUBMIT"]').click();
      })
  }
  else if (globalState.get("connectorId") === "nmi" || globalState.get("connectorId") === "noon") {
    cy.get('iframe', { timeout: 150000 })
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('iframe', { timeout: 20000 })
          .its('0.contentDocument.body')
          .within((body) => {
            cy.get('form[name="cardholderInput"]', { timeout: 20000 }).should('exist').then(form => {
              cy.get('input[name="challengeDataEntry"]').click().type("1234");
              cy.get('input[value="SUBMIT"]').click();
            })
          })
      })
  }
  else if (globalState.get("connectorId") === "stripe") {
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('iframe')
          .its('0.contentDocument.body')
          .within((body) => {
            cy.get('#test-source-authorize-3ds').click();
          })
      })
  }
  else if (globalState.get("connectorId") === "trustpay") {
    cy.get('form[name="challengeForm"]', { timeout: 10000 }).should('exist').then(form => {
      cy.get('#outcomeSelect').select('Approve').should('have.value', 'Y')
      cy.get('button[type="submit"]').click();
    })
  }


  else {
    // If connectorId is neither of adyen, trustpay, nmi, stripe, bankofamerica or cybersource, wait for 10 seconds
    cy.wait(10000);
  }

  // Handling redirection
  if (redirection_url.host.endsWith(expected_url.host)) {
    // No CORS workaround needed
    cy.window().its('location.origin').should('eq', expected_url.origin);
  } else {
    // Workaround for CORS to allow cross-origin iframe
    cy.origin(expected_url.origin, { args: { expected_url: expected_url.origin } }, ({ expected_url }) => {
      cy.window().its('location.origin').should('eq', expected_url);
    })
  }

});

Cypress.Commands.add("listCustomerPMCallTest", (globalState) => {
  console.log("customerID ->" + globalState.get("customerId"));
  const customerId = globalState.get("customerId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customerId}/payment_methods`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");
    if (response.body.customer_payment_methods[0]?.payment_token) {
      const paymentToken = response.body.customer_payment_methods[0].payment_token;
      globalState.set("paymentToken", paymentToken); // Set paymentToken in globalState
      expect(paymentToken).to.equal(globalState.get("paymentToken")); // Verify paymentToken
    }
    else {
      throw new Error(`Payment token not found`);
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
    logRequestId(response.headers['x-request-id']);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.data).to.be.an('array').and.not.empty;
  
    });
  });

  
