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
import { handleRedirection } from "./redirectionHandler";

function logRequestId(xRequestId) {
  if (xRequestId) {
    cy.task("cli_log", "x-request-id -> " + xRequestId);
  } else {
    cy.task("cli_log", "x-request-id is not available in the response headers");
  }
}

function defaultErrorHandler(response, response_data) {
  expect(response.body).to.have.property("error");
  for (const key in response_data.body.error) {
    expect(response_data.body.error[key]).to.equal(response.body.error[key]);
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
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("adminApiKey"),
      },
      body: merchantCreateBody,
    }).then((response) => {
      logRequestId(response.headers["x-request-id"]);

      // Handle the response as needed
      globalState.set("publishableKey", response.body.publishable_key);
    });
  },
);

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
    logRequestId(response.headers["x-request-id"]);

    // Handle the response as needed
    globalState.set("apiKey", response.body.api_key);
  });
});

Cypress.Commands.add(
  "createConnectorCallTest",
  (createConnectorBody, globalState) => {
    const merchantId = globalState.get("merchantId");
    createConnectorBody.connector_name = globalState.get("connectorId");
    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const authDetails = getValueByKey(
          JSON.stringify(jsonContent),
          globalState.get("connectorId"),
        );
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
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(globalState.get("connectorId")).to.equal(
              response.body.connector_name,
            );
          } else {
            cy.task(
              "cli_log",
              "response status -> " + JSON.stringify(response.status),
            );
          }
        });
      },
    );
  },
);

Cypress.Commands.add(
  "createPayoutConnectorCallTest",
  (createConnectorBody, globalState) => {
    const merchantId = globalState.get("merchantId");
    let connectorName = globalState.get("connectorId");
    createConnectorBody.connector_name = connectorName;
    createConnectorBody.connector_type = "payout_processor";
    // readFile is used to read the contents of the file and it always returns a promise ([Object Object]) due to its asynchronous nature
    // it is best to use then() to handle the response within the same block of code
    cy.readFile(globalState.get("connectorAuthFilePath")).then(
      (jsonContent) => {
        const authDetails = getValueByKey(
          JSON.stringify(jsonContent),
          `${connectorName}_payout`,
        );
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
          failOnStatusCode: false,
        }).then((response) => {
          logRequestId(response.headers["x-request-id"]);

          if (response.status === 200) {
            expect(globalState.get("connectorId")).to.equal(
              response.body.connector_name,
            );
          } else {
            cy.task(
              "cli_log",
              "response status -> " + JSON.stringify(response.status),
            );
          }
        });
      },
    );
  },
);

function getValueByKey(jsonObject, key) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;
  if (data && typeof data === "object" && key in data) {
    return data[key];
  } else {
    return null;
  }
}

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
      logRequestId(response.headers["x-request-id"]);

      globalState.set("customerId", response.body.customer_id);
    });
  },
);

Cypress.Commands.add(
  "createPaymentIntentTest",
  (
    request,
    req_data,
    res_data,
    authentication_type,
    capture_method,
    globalState,
  ) => {
    if (
      !request ||
      typeof request !== "object" ||
      !req_data.currency ||
      !authentication_type
    ) {
      throw new Error(
        "Invalid parameters provided to createPaymentIntentTest command",
      );
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
      logRequestId(response.headers["x-request-id"]);

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        expect(response.body).to.have.property("client_secret");
        const clientSecret = response.body.client_secret;
        globalState.set("clientSecret", clientSecret);
        globalState.set("paymentID", response.body.payment_id);
        cy.log(clientSecret);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
        expect(request.amount).to.equal(response.body.amount);
        expect(null).to.equal(response.body.amount_received);
        expect(request.amount).to.equal(response.body.amount_capturable);
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
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
    globalState.set("paymentID", paymentIntentID);
    cy.log(response);
  });
});

Cypress.Commands.add(
  "confirmCallTest",
  (confirmBody, req_data, res_data, confirm, globalState) => {
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
      logRequestId(response.headers["x-request-id"]);
      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        globalState.set("paymentID", paymentIntentID);
        if (response.body.capture_method === "automatic") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url,
            );
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            defaultErrorHandler(response, res_data);
          }
        } else if (response.body.capture_method === "manual") {
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url,
            );
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            defaultErrorHandler(response, res_data);
          }
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
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
      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      globalState.set("paymentID", paymentIntentId);
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
                  response.body.next_action.redirect_to_url,
                );
              }
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                res_data.body.error_code,
              );
            }
          } else {
            defaultErrorHandler(response, res_data);
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
              response.body.next_action.redirect_to_url,
            );
          } else {
            defaultErrorHandler(response, res_data);
          }
          break;
        default:
          defaultErrorHandler(response, res_data);
      }
    });
  },
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
      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      globalState.set("paymentID", paymentIntentID);
      if (response.status === 200) {
        if (
          response.body.capture_method === "automatic" ||
          response.body.capture_method === "manual"
        ) {
          switch (response.body.payment_method_type) {
            case "pix":
              expect(response.body)
                .to.have.property("next_action")
                .to.have.property("qr_code_url");
              globalState.set(
                "nextActionUrl", // This is intentionally kept as nextActionUrl to avoid issues during handleRedirection call,
                response.body.next_action.qr_code_url,
              );
              break;
            default:
              expect(response.body)
                .to.have.property("next_action")
                .to.have.property("redirect_to_url");
              globalState.set(
                "nextActionUrl",
                response.body.next_action.redirect_to_url,
              );
              break;
          }
        } else {
          defaultErrorHandler(response, res_data);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
);

Cypress.Commands.add(
  "createConfirmPaymentTest",
  (
    createConfirmPaymentBody,
    req_data,
    res_data,
    authentication_type,
    capture_method,
    globalState,
  ) => {
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
      logRequestId(response.headers["x-request-id"]);

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        if (response.body.capture_method === "automatic") {
          expect(response.body).to.have.property("status");
          globalState.set("paymentAmount", createConfirmPaymentBody.amount);
          globalState.set("paymentID", response.body.payment_id);
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url,
            );
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            defaultErrorHandler(response, res_data);
          }
        } else if (response.body.capture_method === "manual") {
          expect(response.body).to.have.property("status");
          globalState.set("paymentAmount", createConfirmPaymentBody.amount);
          globalState.set("paymentID", response.body.payment_id);
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url,
            );
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          } else {
            defaultErrorHandler(response, res_data);
          }
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
);

// This is consequent saved card payment confirm call test(Using payment token)
Cypress.Commands.add(
  "saveCardConfirmCallTest",
  (saveCardConfirmBody, req_data, res_data, globalState) => {
    const paymentIntentID = globalState.get("paymentID");
    saveCardConfirmBody.card_cvc = req_data.card.card_cvc;
    saveCardConfirmBody.payment_token = globalState.get("paymentToken");
    saveCardConfirmBody.client_secret = globalState.get("clientSecret");
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

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      globalState.set("paymentID", paymentIntentID);
      if (response.status === 200) {
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
              globalState.get("customerId"),
            );
          } else {
            // Handle other authentication types as needed
            defaultErrorHandler(response, res_data);
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
              globalState.get("customerId"),
            );
          } else {
            // Handle other authentication types as needed
            defaultErrorHandler(response, res_data);
          }
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
);

Cypress.Commands.add(
  "captureCallTest",
  (requestBody, req_data, res_data, amount_to_capture, globalState) => {
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
      logRequestId(response.headers["x-request-id"]);

      expect(res_data.status).to.equal(response.status);
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
  },
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

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
);

Cypress.Commands.add("retrievePaymentCallTest", (globalState) => {
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
    logRequestId(response.headers["x-request-id"]);

    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.payment_id).to.equal(payment_id);
    expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
    globalState.set("paymentID", response.body.payment_id);
  });
});

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
      expect(res_data.status).to.equal(response.status);
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
  },
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
      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      for (const key in res_data.body) {
        expect(res_data.body[key]).to.equal(response.body[key]);
      }
    });
  },
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
    globalState,
  ) => {
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
      logRequestId(response.headers["x-request-id"]);
      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");
      globalState.set("mandateId", response.body.mandate_id);
      globalState.set("paymentID", response.body.payment_id);

      if (response.status === 200) {
        if (response.body.capture_method === "automatic") {
          expect(response.body).to.have.property("mandate_id");
          if (response.body.authentication_type === "three_ds") {
            expect(response.body)
              .to.have.property("next_action")
              .to.have.property("redirect_to_url");
            const nextActionUrl = response.body.next_action.redirect_to_url;
            globalState.set(
              "nextActionUrl",
              response.body.next_action.redirect_to_url,
            );
            cy.log(response.body);
            cy.log(nextActionUrl);
          } else if (response.body.authentication_type === "no_three_ds") {
            for (const key in res_data.body) {
              expect(res_data.body[key]).to.equal(response.body[key]);
            }
          }
          for (const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }
        } else if (response.body.capture_method === "manual") {
          expect(response.body).to.have.property("mandate_id");
          if (response.body.authentication_type === "three_ds") {
            expect(response.body).to.have.property("next_action");
          }
          for (const key in res_data.body) {
            expect(res_data.body[key]).to.equal(response.body[key]);
          }
        }
      } else {
        defaultErrorHandler(response, res_data);
      }
    });
  },
);

Cypress.Commands.add(
  "mitForMandatesCallTest",
  (requestBody, amount, confirm, capture_method, globalState) => {
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
      globalState.set("paymentID", response.body.payment_id);
      if (response.body.capture_method === "automatic") {
        if (response.body.authentication_type === "three_ds") {
          expect(response.body)
            .to.have.property("next_action")
            .to.have.property("redirect_to_url");
          const nextActionUrl = response.body.next_action.redirect_to_url;
          cy.log(response.body);
          cy.log(nextActionUrl);
        } else if (response.body.authentication_type === "no_three_ds") {
          expect(response.body.status).to.equal("succeeded");
        } else {
          defaultErrorHandler(response, res_data);
        }
      } else if (response.body.capture_method === "manual") {
        if (response.body.authentication_type === "three_ds") {
          expect(response.body)
            .to.have.property("next_action")
            .to.have.property("redirect_to_url");
          const nextActionUrl = response.body.next_action.redirect_to_url;
          cy.log(response.body);
          cy.log(nextActionUrl);
        } else if (response.body.authentication_type === "no_three_ds") {
          expect(response.body.status).to.equal("requires_capture");
        } else {
          defaultErrorHandler(response, res_data);
        }
      }
    });
  },
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
      null,
    );
  },
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
        payment_method_type,
      );
    }
  },
);

Cypress.Commands.add(
  "handleBankTransferRedirection",
  (globalState, payment_method_type, expected_redirection) => {
    let connectorId = globalState.get("connectorId");
    let redirection_url = new URL(globalState.get("nextActionUrl"));
    cy.log(payment_method_type);
    handleRedirection(
      "bank_transfer",
      { redirection_url, expected_redirection },
      connectorId,
      payment_method_type,
    );
  },
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
      globalState.set("paymentToken", paymentToken); // Set paymentToken in globalState
      expect(paymentToken).to.equal(globalState.get("paymentToken")); // Verify paymentToken
    } else {
      defaultErrorHandler(response, res_data);
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
    globalState,
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

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        globalState.set("payoutAmount", createConfirmPayoutBody.amount);
        globalState.set("payoutID", response.body.payout_id);
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        expect(response.body).to.have.property("error");
        for (const key in res_data.body.error) {
          expect(res_data.body.error[key]).to.equal(response.body.error[key]);
        }
      }
    });
  },
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

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        expect(response.body).to.have.property("error");
        for (const key in res_data.body.error) {
          expect(res_data.body.error[key]).to.equal(response.body.error[key]);
        }
      }
    });
  },
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

      expect(res_data.status).to.equal(response.status);
      expect(response.headers["content-type"]).to.include("application/json");

      if (response.status === 200) {
        for (const key in res_data.body) {
          expect(res_data.body[key]).to.equal(response.body[key]);
        }
      } else {
        expect(response.body).to.have.property("error");
        for (const key in res_data.body.error) {
          expect(res_data.body.error[key]).to.equal(response.body.error[key]);
        }
      }
    });
  },
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
