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
import ConnectorAuthDetails from "/Users/preetam.revankar/Downloads/creds.json";



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

    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    // Handle the response as needed
    console.log(response.body);
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

    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    // Handle the response as needed
    console.log(response.body);
    globalState.set("apiKey", response.body.api_key);
  });
});

Cypress.Commands.add("createConnectorCallTest", (createConnectorBody, globalState) => {
  const merchantId = globalState.get("merchantId");
  createConnectorBody.connector_name = globalState.get("connectorId");
  const authDetails = getValueByKey(ConnectorAuthDetails, globalState.get("connectorId"));
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
    // Handle the response as needed
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }

    if (response.status === 200) {
      expect(globalState.get("connectorId")).to.equal(response.body.connector_name);
    }
    else {
      cy.task('cli_log', "response status ->> " + JSON.stringify(response.status));
      cy.task('cli_log', "res body ->> " + JSON.stringify(response.body));
    }

    
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

    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    // Handle the response as needed
    console.log(response);

    globalState.set("customerId", response.body.customer_id);
  });
});

Cypress.Commands.add("createPaymentIntentTest", (request, det, authentication_type, capture_method, globalState) => {
  if (!request || typeof request !== "object" || !det.currency || !authentication_type) {
    throw new Error("Invalid parameters provided to createPaymentIntentTest command");
  }
  request.currency = det.currency;
  request.authentication_type = authentication_type;
  request.capture_method = capture_method;
  request.setup_future_usage= det.setup_future_usage; 
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
    body: request,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("client_secret");
    const clientSecret = response.body.client_secret;
    globalState.set("clientSecret", clientSecret);
    globalState.set("paymentID", response.body.payment_id);
    cy.log(clientSecret);
    expect("requires_payment_method").to.equal(response.body.status);
    expect(request.amount).to.equal(response.body.amount);
    expect(null).to.equal(response.body.amount_received);
    expect(request.amount).to.equal(response.body.amount_capturable);
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
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    console.log(response);
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("redirect_url");
    expect(response.body).to.have.property("payment_methods");
    globalState.set("paymentID", paymentIntentID);
    cy.log(response);
  });
});

Cypress.Commands.add("confirmCallTest", (confirmBody, details, confirm, globalState) => {

  const paymentIntentID = globalState.get("paymentID");
  confirmBody.payment_method_data.card = details.card;
  confirmBody.confirm = confirm;
  confirmBody.client_secret = globalState.get("clientSecret");
  confirmBody.customer_acceptance = details.customer_acceptance;

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
    body: confirmBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    globalState.set("paymentID", paymentIntentID);
    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(details.paymentSuccessfulStatus).to.equal(response.body.status);
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
        expect("requires_capture").to.equal(response.body.status);
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else {
      throw new Error(`Unsupported capture method: ${capture_method}`);
    }
  });
});

Cypress.Commands.add("createConfirmPaymentTest", (createConfirmPaymentBody, details, authentication_type, capture_method, globalState) => {
  createConfirmPaymentBody.payment_method_data.card = details.card;
  createConfirmPaymentBody.authentication_type = authentication_type;
  createConfirmPaymentBody.currency = details.currency;
  createConfirmPaymentBody.capture_method = capture_method;
  createConfirmPaymentBody.customer_acceptance = details.customer_acceptance;
  createConfirmPaymentBody.setup_future_usage = details.setup_future_usage;
  createConfirmPaymentBody.customer_id = globalState.get("customerId");

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: createConfirmPaymentBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("status");
    console.log(response.body);
    globalState.set("paymentAmount", createConfirmPaymentBody.amount);
    globalState.set("paymentID", response.body.payment_id);
    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url")
          globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
      }
      else if (response.body.authentication_type === "no_three_ds") {
        expect(details.paymentSuccessfulStatus).to.equal(response.body.status);
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else if (response.body.capture_method === "manual") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url")
          globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
      }
      else if (response.body.authentication_type === "no_three_ds") {
        expect("requires_capture").to.equal(response.body.status);
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
  });
});

// This is consequent saved card payment confirm call test(Using payment token)
Cypress.Commands.add("saveCardConfirmCallTest", (confirmBody,det,globalState) => {
  const paymentIntentID = globalState.get("paymentID");
  confirmBody.card_cvc = det.card.card_cvc;
  confirmBody.payment_token = globalState.get("paymentToken");
  confirmBody.client_secret = globalState.get("clientSecret");
  console.log("conf conn ->" + globalState.get("connectorId"));
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}/confirm`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
    body: confirmBody,
  })
  .then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    globalState.set("paymentID", paymentIntentID);
    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal(det.paymentSuccessfulStatus);
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
        expect(response.body.status).to.equal("requires_capture");
        expect(response.body.customer_id).to.equal(globalState.get("customerId"));
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else {
      throw new Error(`Unsupported capture method: ${capture_method}`);
    }
  });
});


Cypress.Commands.add("captureCallTest", (requestBody, amount_to_capture, paymentSuccessfulStatus, globalState) => {
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
    body: requestBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.payment_id).to.equal(payment_id);
    console.log(response.body);
    if (amount_to_capture == amount && response.body.status == "succeeded") {
      expect(response.body.amount).to.equal(amount_to_capture);
      expect(response.body.amount_capturable).to.equal(0);
      expect(response.body.amount_received).to.equal(amount);
      expect(response.body.status).to.equal(paymentSuccessfulStatus);
    }else if (response.body.status=="processing") {
      expect(response.body.amount).to.equal(amount);
      expect(response.body.amount_capturable).to.equal(amount);
      expect(response.body.amount_received).to.equal(0);
      expect(response.body.status).to.equal(paymentSuccessfulStatus);
    }
     else {
      expect(response.body.amount).to.equal(amount);
      expect(response.body.amount_capturable).to.equal(0);
      expect(response.body.amount_received).to.equal(amount_to_capture);
      expect(response.body.status).to.equal("partially_captured");
    }
  });
});

Cypress.Commands.add("voidCallTest", (requestBody, globalState) => {
  const payment_id = globalState.get("paymentID");
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}/cancel`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.payment_id).to.equal(payment_id);
    expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
    // expect(response.body.amount_capturable).to.equal(0);
    expect(response.body.amount_received).to.be.oneOf([0, null]);
    expect(response.body.status).to.equal("cancelled");
    console.log(response.body);
  });
});

Cypress.Commands.add("retrievePaymentCallTest", (globalState) => {
  console.log("syncpaymentID------>" + globalState.get("paymentID"));
  const payment_id = globalState.get("paymentID");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    expect(response.body.payment_id).to.equal(payment_id);
    expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
    globalState.set("paymentID", response.body.payment_id);

  });
});

Cypress.Commands.add("refundCallTest", (requestBody, refund_amount, det, globalState) => {
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
    body: requestBody
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    globalState.set("refundId", response.body.refund_id);
    expect(response.body.status).to.equal(det.refundStatus);
    expect(response.body.amount).to.equal(refund_amount);
    expect(response.body.payment_id).to.equal(payment_id);
  });
});

Cypress.Commands.add("syncRefundCallTest", (det, globalState) => {
  const refundId = globalState.get("refundId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/refunds/${refundId}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    expect(response.body.status).to.equal(det.refundSyncStatus);
  });
});

Cypress.Commands.add("citForMandatesCallTest", (requestBody, authentication_type ,amount, details, confirm, capture_method, payment_type, globalState) => {
  requestBody.authentication_type = authentication_type;
  requestBody.payment_method_data.card = details.card;
  requestBody.payment_type=payment_type;
  requestBody.confirm = confirm;
  requestBody.amount = amount;
  requestBody.currency = details.currency;
  requestBody.capture_method = capture_method;
  requestBody.mandate_data.mandate_type = details.mandate_type;
  requestBody.customer_id = globalState.get("customerId");
  globalState.set("paymentAmount", requestBody.amount);
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("mandate_id");
    console.log(response.body);
    globalState.set("mandateId", response.body.mandate_id);
    globalState.set("paymentID", response.body.payment_id);

    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
        globalState.set("nextActionUrl", response.body.next_action.redirect_to_url);
        cy.log(response.body);
        cy.log(nextActionUrl)
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal(details.paymentSuccessfulStatus);
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else if (response.body.capture_method === "manual") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
      }
      else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal("requires_capture");
      } else {
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
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
    body: requestBody,
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    globalState.set("paymentID", response.body.payment_id);
    console.log(response.body);
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
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
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
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    if (response.body.status === 200) {
      expect(response.body.status).to.equal("revoked");
    } else if (response.body.status === 400) {
      expect(response.body.reason).to.equal("Mandate has already been revoked");
    }
  });
});


Cypress.Commands.add("handleRedirection", (globalState, expected_redirection) => {
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
  else if (globalState.get("connectorId") === "cybersource" || globalState.get("connectorId") === "bankofamerica" ) {
    
    
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('input[type="text"]').click().type("1234");
        cy.get('input[value="SUBMIT"]').click();
  })
  }
  else if (globalState.get("connectorId") === "nmi" || globalState.get("connectorId") === "noon") {
    cy.get('iframe',{ timeout: 100000 })
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('iframe',{ timeout: 10000 })
          .its('0.contentDocument.body')
          .within((body) => {
        cy.get('form[name="cardholderInput"]',{ timeout: 10000 }).should('exist').then(form => {
          cy.get('input[name="challengeDataEntry"]').click().type("1234");
          cy.get('input[value="SUBMIT"]').click();
          })
      })
    })
  }
  else if (globalState.get("connectorId") === "stripe" ) {
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
  else if (globalState.get("connectorId") === "trustpay" ) {
    cy.get('form[name="challengeForm"]',{ timeout: 10000 }).should('exist').then(form => {
        cy.get('#outcomeSelect').select('Approve').should('have.value', 'Y')
        cy.get('button[type="submit"]').click();
  })
  }

  
  else {
  // If connectorId is neither of adyen, trustpay, nmi, stripe, bankofamerica or cybersource, wait for 30 seconds
    cy.wait(30000);
  }
  
  // Handling redirection
  if (redirection_url.host.endsWith(expected_url.host)) {
    // No CORS workaround needed
    cy.window().its('location.origin').should('eq', expected_url.origin);
  } else {
    // Workaround for CORS to allow cross-origin iframe
    cy.origin(expected_url.origin, { args: { expected_url: expected_url.origin} }, ({expected_url}) => {
      cy.window().its('location.origin').should('eq', expected_url);
    })
  }
  
});

Cypress.Commands.add("listCustomerPMCallTest", (globalState) => {
  console.log("customerID------>" + globalState.get("customerId"));
  const customerId = globalState.get("customerId");
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/customers/${customerId}/payment_methods`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
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

Cypress.Commands.add("listRefundCallTest", (globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/refunds/list`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body:{"offset":0}
  }).then((response) => {
    const xRequestId = response.headers['x-request-id'];
    if (xRequestId) {
      cy.task('cli_log', "x-request-id ->> " + xRequestId);
    } else {
      cy.task('cli_log', "x-request-id is not available in the response headers");
    }
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    expect(response.body.data).to.be.an('array').and.not.empty;
  
    });
  });