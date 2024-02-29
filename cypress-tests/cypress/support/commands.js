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
// import globalState from "../utils/State";
import * as RequestBodyUtils from "../utils/RequestBodyUtils";
import { baseUrl } from "../utils/Constants";
import ConnectorAuthDetails from "../../../.github/secrets/creds.json";
cy.task('cli_log', "ConnectorAuthDetails -> " + JSON.stringify(ConnectorAuthDetails));
console.log(JSON.stringify(ConnectorAuthDetails));

const adminApiKey = ConnectorAuthDetails.integ.ADMIN_API_KEYS ;

Cypress.Commands.add("merchantCreateCallTest", (merchantCreateBody, globalState) => {

  const randomMerchantId = RequestBodyUtils.generateRandomString();
  RequestBodyUtils.setMerchantId(merchantCreateBody, randomMerchantId);
  console.log("globalState -->" + JSON.stringify(globalState));
  console.log(typeof globalState);
  globalState.set("merchantId", randomMerchantId);
  console.log("merchantid------>" + globalState.get("merchantId"));

  cy.request({
    method: "POST",
    url: `${baseUrl}/accounts`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": adminApiKey,
    },
    body: merchantCreateBody,
  }).then((response) => {
    // Handle the response as needed
    console.log(response.body);
    globalState.set("publishableKey", response.body.publishable_key);
    console.log("publishable_key------>" + globalState.get("publishableKey"));
  });
});

Cypress.Commands.add("apiKeyCreateTest", (apiKeyCreateBody, globalState) => {
  cy.request({
    method: "POST",
    url: `${baseUrl}/api_keys/${globalState.get("merchantId")}`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": adminApiKey,
    },
    body: apiKeyCreateBody,
  }).then((response) => {
    // Handle the response as needed
    console.log(response.body.api_key);
    globalState.set("apiKey", response.body.api_key);
    console.log("api_key------>" + globalState.get("apiKey"));
  });
});

Cypress.Commands.add("createConnectorCallTest", (createConnectorBody, globalState) => {
  // RequestBodyUtils.setApiKey(createConnectorBody, globalState.get("apiKey"));
  const merchantId = globalState.get("merchantId");
  console.log("merchantid-------->" + merchantId);
  createConnectorBody.connector_name = globalState.get("connectorId");
  console.log("connn ->" + globalState.get("connectorId"));
  const authDetails = getValueByKey(ConnectorAuthDetails, globalState.get("connectorId"));
  console.log("authDetails-------->" + authDetails);
  createConnectorBody.connector_account_details = authDetails;
  console.log("createConnectorBody-------->" + createConnectorBody);
  cy.request({
    method: "POST",
    url: `${baseUrl}/account/${merchantId}/connectors`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": adminApiKey,
    },
    body: createConnectorBody,
  }).then((response) => {
    // Handle the response as needed
    console.log(response.body);
  });
});

function getValueByKey(jsonObject, key) {
  // Convert the input JSON string to a JavaScript object if it's a string
  const data = typeof jsonObject === 'string' ? JSON.parse(jsonObject) : jsonObject;

  // Check if the key exists in the object
  if (data && typeof data === 'object' && key in data) {
    return data[key];
  } else {
    return null; // Key not found
  }
}

Cypress.Commands.add("createPaymentIntentTest", (request, currency, authentication_type, capture_method, globalState) => {
  console.log("cl intent------>");
  if (!request || typeof request !== "object" || !currency || !authentication_type) {
    throw new Error("Invalid parameters provided to createPaymentIntentTest command");
  }
  console.log("int connn ->" + globalState.get("connectorId"));
  request.currency = currency;
  request.authentication_type = authentication_type;
  request.capture_method = capture_method;
  console.log("api_key ------>" + globalState.get("apiKey"));
  globalState.set("paymentAmount", request.amount);
  cy.request({
    method: "POST",
    url: `${baseUrl}/payments`,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: request,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("client_secret");
    const clientSecret = response.body.client_secret;
    globalState.set("clientSecret", clientSecret);
    cy.log(clientSecret);
  });
});

Cypress.Commands.add("paymentMethodsCallTest", (globalState) => {
  const clientSecret = globalState.get("clientSecret");

  cy.request({
    method: "GET",
    url: `${baseUrl}/account/payment_methods?client_secret=${clientSecret}`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
  }).then((response) => {
    console.log(response);
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("redirect_url");
    expect(response.body).to.have.property("payment_methods");
    cy.log(response);
  });
});

Cypress.Commands.add("confirmCallTest", (confirmBody, details, confirm, globalState) => {
  const clientSecret = globalState.get("clientSecret");
  var paymentIntentID = clientSecret.split("_secret_")[0];

  console.log("cl confirm------>" + clientSecret);

  // RequestBodyUtils.setCardNo(confirmBody, details.card);
  confirmBody.payment_method_data.card = details.card;
  confirmBody.confirm = confirm;
  RequestBodyUtils.setClientSecret(confirmBody, clientSecret);
  console.log("conf connn ->" + globalState.get("connectorId"));

  cy.request({
    method: "POST",
    url: `${baseUrl}/payments/${paymentIntentID}/confirm`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("publishableKey"),
    },
    body: confirmBody,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    console.log(response.body);
    globalState.set("paymentId", response.body.payment_id);
    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
        cy.task('cli_log', "nextActionUrl -> " + nextActionUrl);
        cy.log(response.body);
        cy.log(nextActionUrl);
        console.log(nextActionUrl);
        let url = new URL(nextActionUrl);
        const args = { nextActionUrl, details };
        cy.origin(url.host, { args }, ({ nextActionUrl, details }) => {
          // let nexturl = new URL(nextActionUrl);
          cy.visit(nextActionUrl)
          cy.wait(5000);
          cy.contains('button', 'Submit').click('.btn');
          console.log("buttonn visited------>");
          let updated_url = cy.url().then(url => { console.log(url) });
          console.log("updated_url------>" + updated_url);
          cy.url().should('eq', 'https://hyperswitch.io/');
        })
        // cy.window().invoke("open", nextActionUrl);
        // expect(response.body.status).to.equal(details.successfulStates);
        // cy.visit(nextActionUrl);
        // cy.wait(5000);
        // cy.url().should('eq', 'https://hyperswitch.io/');
        // console.log("visited------>" + nextActionUrl);
        // cy.contains('button', 'Complete').click('.test-source-authorize-3ds');
        // // Switch to the new window
        // cy.window().then((newWindow) => {
        //   // Add a wait if needed for the new window to fully load
        //   cy.wait(500); // Adjust the wait time as needed
        //   console.log("new blah yet visited------>" );
        //   // Perform actions in the new window
        //   newWindow.document.getElementById('test-source-authorize-3ds').click();

        //   // Switch back to the original window if needed
        //   cy.window().then((originalWindow) => {
        //     // Continue with actions in the original window
        //     originalWindow.document.getElementById('test-source-authorize-3ds').click();
        //     console.log("blah visited------>");
        //   });
        // });
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal(details.successfulStates);
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    } else if (response.body.capture_method === "manual") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
      }
      else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal("requires_capture");
      } else {
        // Handle other authentication types as needed
        throw new Error(`Unsupported authentication type: ${authentication_type}`);
      }
    }
    else {
      throw new Error(`Unsupported capture method: ${capture_method}`);
    }

    //   if (response.body.authentication_type === "three_ds") {
    //     expect(response.body).to.have.property("next_action")
    //       .to.have.property("redirect_to_url");

    //     const nextActionUrl = response.body.next_action.redirect_to_url;
    //     cy.log(response.body);
    //     cy.log(nextActionUrl);

    //     // Use cy.request to follow the redirect and get the content of the new page
    //     cy.request(nextActionUrl).then((redirectedResponse) => {
    //       cy.wait(5000);
    //       cy.log(redirectedResponse.body);

    //       // Find and interact with the button in the new page
    //       cy.get('#test-source-authorize-3ds').click();
    //       console.log("blah visited------>");

    //       // Continue with other assertions or actions as needed
    //     });
    //   } else if (response.body.authentication_type === "no_three_ds") {
    //     expect(response.body.status).to.equal(details.successfulStates);
    //   } else {
    //     // Handle other authentication types as needed
    //     throw new Error(`Unsupported authentication type: ${authentication_type}`);
    //   }
  });
});


Cypress.Commands.add("captureCallTest", (requestBody, amount_to_capture, successfulStates, globalState) => {
  let payment_id = globalState.get("paymentId");
  requestBody.amount_to_capture = amount_to_capture;
  let amount = globalState.get("paymentAmount");
  console.log("amount------>" + amount);
  cy.request({
    method: "POST",
    url: `${baseUrl}/payments/${payment_id}/capture`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    if (amount_to_capture == amount) {
      expect(response.body.status).to.equal(successfulStates);
    } else {
      expect(response.body.status).to.equal("partially_captured");
    }



  });
});

Cypress.Commands.add("voidCallTest", (requestBody, globalState) => {
  const clientSecret = globalState.get("clientSecret");
  var paymentIntentID = clientSecret.split("_secret_")[0];
  cy.request({
    method: "POST",
    url: `${baseUrl}/payments/${paymentIntentID}/cancel`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body.status).to.equal("cancelled");
    console.log(response.body);
  });
});

Cypress.Commands.add("retrievePaymentCallTest", (globalState) => {
  const clientSecret = globalState.get("clientSecret");
  var paymentIntentID = clientSecret.split("_secret_")[0];
  cy.request({
    method: "GET",
    url: `${baseUrl}/payments/${paymentIntentID}?force_sync=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    console.log("sync status -->>" + response.body.status);
    console.log(response.body);
  });
});

Cypress.Commands.add("refundCallTest", (requestBody, refund_amount, globalState) => {
  const clientSecret = globalState.get("clientSecret");
  var paymentIntentID = clientSecret.split("_secret_")[0];
  requestBody.payment_id = paymentIntentID;
  requestBody.amount = refund_amount;
  cy.request({
    method: "POST",
    url: `${baseUrl}/refunds`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    console.log("sync status -->>" + response.body.status);
    console.log(response.body);
    globalState.set("refundId", response.body.refund_id);
    expect(response.body.status).to.equal("succeeded");
  });
});

Cypress.Commands.add("syncRefundCallTest", (globalState) => {
  const refund_id = globalState.get("refundId");
  cy.request({
    method: "GET",
    url: `${baseUrl}/refunds/${refund_id}?force_sync=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    console.log("sync status -->>" + response.body.status);
    console.log(response.body);
    expect(response.body.status).to.equal("succeeded");
  });
});

Cypress.Commands.add("citForMandatesCallTest", (requestBody, currency, details, confirm, capture_method, globalState) => {

  requestBody.payment_method_data.card = details.card;
  requestBody.confirm = confirm;
  requestBody.currency = currency;
  requestBody.capture_method = capture_method;
  requestBody.mandate_data.mandate_type = details.mandate_type;
  globalState.set("paymentAmount", requestBody.amount);
  cy.request({
    method: "POST",
    url: `${baseUrl}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    expect(response.body).to.have.property("mandate_id");
    console.log("mandate_id -->>" + response.body.mandate_id);
    globalState.set("mandateId", response.body.mandate_id);
    globalState.set("paymentId", response.body.payment_id);

    if (response.body.capture_method === "automatic") {
      if (response.body.authentication_type === "three_ds") {
        expect(response.body).to.have.property("next_action")
          .to.have.property("redirect_to_url");
        const nextActionUrl = response.body.next_action.redirect_to_url;
        cy.log(response.body);
        cy.log(nextActionUrl);
      } else if (response.body.authentication_type === "no_three_ds") {
        expect(response.body.status).to.equal(details.successfulStates);
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
        // Handle other authentication types as needed
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
  globalState.set("paymentAmount", requestBody.amount);
  cy.request({
    method: "POST",
    url: `${baseUrl}/payments`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    body: requestBody,
  }).then((response) => {
    expect(response.headers["content-type"]).to.include("application/json");
    globalState.set("paymentId", response.body.payment_id);
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


// const fs = require('fs');
// const path = require('path');

// Cypress.Commands.add('cliLogToFile', (message) => {
//   // Specify the parent directory where you want to save the log file
//   const parentDirectory = '../../';  // Adjust the relative path as needed
//   const filePath = path.join(__dirname, parentDirectory, 'cliLog.txt');

//   // Log the message to the console
//   cy.log(message);

//   // Append the message to the file
//   fs.appendFileSync(filePath, `${message}\n`);

//   // Log success to the console
//   cy.log(`Message logged to file: ${filePath}`);
// });
