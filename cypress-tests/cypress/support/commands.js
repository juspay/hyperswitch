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
const responseHandler = require("./responseHandler");

export function globalStateSetter(globalState, key, value) {
  globalState.set(key, value);
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
    responseHandler.logRequestId(response.headers['x-request-id']);
    responseHandler.validateExistenceOfMerchantId(globalState.get("merchantId"), response.body.merchant_id);
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    globalState.set("apiKey", response.body.api_key);
  });
});

Cypress.Commands.add("createConnectorCallTest", (createConnectorBody, globalState) => {
  const merchantId = globalState.get("merchantId");
  const connectorId = globalState.get("connectorId");
  createConnectorBody.connector_name = connectorId;
  // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
  // it is best to use then() to handle the response within the same block of code
  cy.readFile(globalState.get("connectorAuthFilePath")).then((jsonContent) => {
    const authDetails = getValueByKey(JSON.stringify(jsonContent), connectorId);
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
      responseHandler.logRequestId(response.headers['x-request-id']);
      responseHandler.validateContentType(response);

      if (response.status === 200) {
        responseHandler.validateConnectorName(response.body.connector_name, connectorId);
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
    responseHandler.logRequestId(response.headers['x-request-id']);
    responseHandler.validateContentType(response);
    globalState.set("customerId", response.body.customer_id);
    console.log(response);
  });
});

Cypress.Commands.add("createPaymentIntentTest", (request, det, authentication_type, capture_method, globalState) => {
  if (!request || typeof request !== "object" || !det.currency || !authentication_type) {
    throw new Error("Invalid parameters provided to createPaymentIntentTest command");
  }
  request.currency = det.currency;
  request.authentication_type = authentication_type;
  request.capture_method = capture_method;
  request.setup_future_usage = det.setup_future_usage;
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
    responseHandler.logRequestId(response.headers['x-request-id']);
    responseHandler.validateContentType(response);
    responseHandler.validateExistenceOfClientSecret(response.body);
    const clientSecret = response.body.client_secret;
    globalState.set("clientSecret", clientSecret);
    globalState.set("paymentID", response.body.payment_id);
    cy.log(clientSecret);
    responseHandler.validateResponseStatus("requires_payment_method", response.body.status);
    responseHandler.validateAmount(request.amount, response);
    responseHandler.validateCapturableAmount(request, response);
    responseHandler.validateReceivedAmount(request.amount, request, response);
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    console.log(response);
    responseHandler.validateContentType(response);
    responseHandler.validateExistenceOfPMRedirectUrl(response);
    responseHandler.validateExistenceOfPaymentMethods(response);
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    globalState.set("paymentID", paymentIntentID);
    responseHandler.handleAuthType(response, globalState, true, details);
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validateExistenceOfStatus(response);
    globalState.set("paymentAmount", createConfirmPaymentBody.amount);
    globalState.set("paymentID", response.body.payment_id);
    responseHandler.handleAuthType(response, globalState, true, details);
  });
});

// This is consequent saved card payment confirm call test(Using payment token)
Cypress.Commands.add("saveCardConfirmCallTest", (confirmBody, det, globalState) => {
  const paymentIntentID = globalState.get("paymentID");
  confirmBody.card_cvc = det.card.card_cvc;
  confirmBody.payment_token = globalState.get("paymentToken");
  confirmBody.client_secret = globalState.get("clientSecret");
  console.log("configured connector ->" + globalState.get("connectorId"));
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
      responseHandler.logRequestId(response.headers['x-request-id']);

      responseHandler.validateContentType(response);
      globalState.set("paymentID", paymentIntentID);
      responseHandler.handleAuthType(response, globalState, true, det);
    });
});

Cypress.Commands.add("captureCallTest", (request, amount_to_capture, paymentSuccessfulStatus, globalState) => {
  const payment_id = globalState.get("paymentID");
  request.amount_to_capture = amount_to_capture;
  let amount = globalState.get("paymentAmount");
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}/capture`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validatePaymentId(response, payment_id);
    // expect(response.body.payment_id).to.equal(payment_id);
    if (amount_to_capture == amount && response.body.status == "succeeded") {
      responseHandler.validateAmount(amount, response);
      responseHandler.validatePaymentId(response, payment_id);
      responseHandler.validateAmountToCapture(response.body.amount, amount_to_capture);
      responseHandler.validateCapturableAmount(request, response);
      responseHandler.validateReceivedAmount(amount, request, response);
      responseHandler.validatePaymentStatus(paymentSuccessfulStatus, response.body.status);
    } else if (response.body.status == "processing") {
      responseHandler.validateAmount(amount, response)
      responseHandler.validateAmountToCapture(response.body.amount, amount_to_capture);
      responseHandler.validateCapturableAmount(request, response);
      responseHandler.validateReceivedAmount(amount, request, response);
      responseHandler.validatePaymentStatus(paymentSuccessfulStatus, response.body.status);
    }
    else {

      expect(response.body.amount).to.equal(amount);
      expect(response.body.amount_capturable).to.equal(0);
      expect(response.body.amount_received).to.equal(amount_to_capture);
      expect(response.body.status).to.equal("partially_captured");

      responseHandler.validateAmount(amount, response);
      responseHandler.validateCapturableAmount(request, response);
      responseHandler.validateReceivedAmount(amount, request, response);
      responseHandler.validatePaymentStatus("partially_captured", response.body.status);
    }
  });
});

Cypress.Commands.add("voidCallTest", (request, globalState) => {
  const payment_id = globalState.get("paymentID");
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}/cancel`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validatePaymentId(response, payment_id);
    responseHandler.validateAmount(globalState.get("paymentAmount"), response);
    responseHandler.validateReceivedAmount(response.body.amount_received, request, response);
    responseHandler.validatePaymentStatus("cancelled", response.body.status);
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
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validatePaymentId(response, payment_id);
    responseHandler.validateAmount(globalState.get("paymentAmount"), response);
    globalState.set("paymentID", response.body.payment_id);

  });
});

Cypress.Commands.add("refundCallTest", (request, refund_amount, det, globalState) => {
  const payment_id = globalState.get("paymentID");
  request.payment_id = payment_id;
  request.amount = refund_amount;
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/refunds`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    globalState.set("refundId", response.body.refund_id);
    responseHandler.validatePaymentStatus(det.refundStatus, response.body.status);
    responseHandler.validateAmount(refund_amount, response);
    responseHandler.validatePaymentId(response, payment_id);
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validatePaymentStatus(det.refundSyncStatus, response.body.status);
  });
});

Cypress.Commands.add("citForMandatesCallTest", (request, amount, details, confirm, capture_method, payment_type, globalState) => {
  request.payment_method_data.card = details.card;
  request.payment_type = payment_type;
  request.confirm = confirm;
  request.amount = amount;
  request.currency = details.currency;
  request.capture_method = capture_method;
  request.mandate_data.mandate_type = details.mandate_type;
  request.customer_id = globalState.get("customerId");
  globalState.set("paymentAmount", request.amount);
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validateExistenceOfMandateId(response);
    globalState.set("mandateId", response.body.mandate_id);
    globalState.set("paymentID", response.body.payment_id);

    responseHandler.handleAuthType(response, globalState, true, details);
  });
});

Cypress.Commands.add("mitForMandatesCallTest", (request, amount, confirm, capture_method, globalState) => {
  request.amount = amount;
  request.confirm = confirm;
  request.capture_method = capture_method;
  request.mandate_id = globalState.get("mandateId");
  request.customer_id = globalState.get("customerId");
  globalState.set("paymentAmount", request.amount);
  console.log("mit body " + JSON.stringify(request));
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    globalState.set("paymentID", response.body.payment_id);
    console.log("mit status -> " + response.body.status);
    responseHandler.handleAuthType(response, globalState, true, { paymentSuccessfulStatus: "succeeded" });
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);

    let i = 0;
    for (i in response.body) {
      if (response.body[i].mandate_id === globalState.get("mandateId")) {
        responseHandler.validateMandateStatus(response.body[i].status, "active");
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    if (response.body.status === 200) {
      responseHandler.validateMandateStatus(response.body.status, "revoked");
    } else if (response.body.status === 400) {
      responseHandler.validateMandateReason(response.body.reason, "Mandate has already been revoked");
    }
  });
});

Cypress.Commands.add("handleRedirection", (globalState, expected_redirection) => {
  let connectorId = globalState.get("connectorId");
  let expected_url = new URL(expected_redirection);
  let redirection_url = new URL(globalState.get("nextActionUrl"));
  cy.visit(redirection_url.href);
  if (connectorId == "adyen") {
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('input[type="password"]').click();
        cy.get('input[type="password"]').type("password");
        cy.get('#buttonSubmit').click();
      })
  }
  else if (connectorId === "cybersource" || connectorId === "bankofamerica") {
    cy.get('iframe')
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('input[type="text"]').click().type("1234");
        cy.get('input[value="SUBMIT"]').click();
      })
  }
  else if (connectorId === "nmi" || connectorId === "noon") {
    cy.get('iframe', { timeout: 100000 })
      .its('0.contentDocument.body')
      .within((body) => {
        cy.get('iframe', { timeout: 10000 })
          .its('0.contentDocument.body')
          .within((body) => {
            cy.get('form[name="cardholderInput"]', { timeout: 10000 }).should('exist').then(form => {
              cy.get('input[name="challengeDataEntry"]').click().type("1234");
              cy.get('input[value="SUBMIT"]').click();
            })
          })
      })
  }
  else if (connectorId === "stripe") {
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
  else if (connectorId === "trustpay") {
    cy.get('form[name="challengeForm"]', { timeout: 10000 }).should('exist').then(form => {
      cy.get('#outcomeSelect').select('Approve').should('have.value', 'Y')
      cy.get('button[type="submit"]').click();
    })
  }


  else {
    // If connectorId is neither of one among mentioned above, wait for 10 seconds
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
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    if (response.body.customer_payment_methods[0]?.payment_token) {
      const paymentToken = response.body.customer_payment_methods[0].payment_token;
      globalState.set("paymentToken", paymentToken); // Set paymentToken in globalState
      responseHandler.validatePaymentToken(globalState.get("paymentToken"), paymentToken); // Verify paymentToken
    }
    else {
      throw new Error(`Payment token not found`);
    }
  });
});

Cypress.Commands.add("listRefundCallTest", (request, globalState) => {
  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/refunds/list`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    responseHandler.logRequestId(response.headers['x-request-id']);

    responseHandler.validateContentType(response);
    responseHandler.validateArrayResponse(response.body.data);
  });
});
