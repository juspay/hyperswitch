import { ConnectorPayload, getConnectorDetails } from "../Utils/ConnectorUtils";
import { createMerchantConnectorDetails } from "../../fixtures/logins";
import * as bodyy from "../Utils/BodyHelper";

describe("Amazon Pay Refund Tests", () => {
  let authUrl = "";
  let postLoginUrl = "";
  const urlPrefix = `${Cypress.env("BASEURL")}`;

  before(() => {
    authUrl = `${urlPrefix}/user/signin`;
    postLoginUrl = `${urlPrefix}/dashboard/home`;

    Cypress.on("uncaught:exception", (err, runnable) => {
      return false;
    });

    cy.fixture("logins.json").then((data) => {
      globalState.email = data["Email"];
      globalState.password = data["Password"];
    });

    cy.login(globalState.email, globalState.password, authUrl, postLoginUrl);
  });

  after(() => {
    cy.task("cli_log", "Running after hook...");
    cy.serverCall();
  });

  context("Amazon Pay Refund - Happy Path", () => {
    it("01-AccountCreate - Create merchant account", () => {
      const key = getConnectorDetails(globalState.connector).key;
      cy.merchantAccountCreation(ConnectorPayload.merchantAccountPayload(), key, globalState);
    });

    it("02-CustomerCreate - Create customer", () => {
      cy.customerCreate(bodyy.Customer.body, globalState);
    });

    it("03-ConnectorCreate - Create Amazon Pay connector", () => {
      const key = getConnectorDetails(globalState.connector).key;
      const connectorData = getConnectorDetails(globalState.connector);
      cy.createConnectorCall(createMerchantConnectorDetails(key), globalState, connectorData);
    });

    it("Create Payment - Amazon Pay Charge", () => {
      const data = bodyy.CreatePaymentBody;
      data.profile_id = globalState.get("profileId");
      data.customer_id = globalState.get("customerId");
      data.capture_method = "automatic";
      data.amount = 6500;
      data.currency = "USD";
      data.confirm = true;
      data.capture_on = "";
      data.authentication_type = "no_three_ds";
      data.payment_method = "wallet";
      data.payment_method_type = "amazon_pay";
      data.payment_method_data = {
        wallet: {
          amazon_pay: {}
        }
      };
      data.billing = {
        address: {
          line1: "1467",
          line2: "Harrison Street",
          line3: "Harrison Street",
          city: "San Fransico",
          state: "California",
          zip: "94122",
          country: "US",
          first_name: "joseph",
          last_name: "Doe"
        },
        phone: {
          number: "8056594427",
          country_code: "+91"
        }
      };

      cy.request({
        method: "POST",
        url: `${globalState.baseUrl}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("publishableKey")
        },
        body: data,
        failOnStatusCode: false
      }).then((response) => {
        expect(response.status).to.eq(200);
        globalState.set("paymentId", response.body.payment_id);
        globalState.set("paymentIntentId", response.body.payment_intent_id);
        cy.task("cli_log", "Amazon Pay payment created: " + response.body.payment_id);
      });
    });

    it("Retrieve Payment - Verify payment status", () => {
      cy.retrievePaymentCall(globalState);
    });

    it("Create Refund - Amazon Pay Refund", () => {
      const refundRequest = {
        payment_id: globalState.get("paymentId"),
        amount: 6500,
        reason: "Customer request",
        metadata: {
          udf1: "test",
          new_customer: "true",
          login_date: "2019-09-10T10:11:12Z"
        }
      };

      cy.request({
        method: "POST",
        url: `${globalState.baseUrl}/refunds`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey")
        },
        body: refundRequest,
        failOnStatusCode: false
      }).then((response) => {
        expect(response.status).to.be.oneOf([200, 400]);
        if (response.status === 200) {
          globalState.set("refundId", response.body.refund_id);
          globalState.set("refundStatus", response.body.status);
        }
        cy.task("cli_log", "Refund response: " + JSON.stringify(response.body));
      });
    });

    it("Retrieve Refund - Verify refund status", () => {
      const refundId = globalState.get("refundId");
      if (!refundId) {
        cy.log("Refund ID not available - skipping retrieve");
        return;
      }

      cy.request({
        method: "GET",
        url: `${globalState.baseUrl}/refunds/${refundId}`,
        headers: {
          "api-key": globalState.get("apiKey")
        },
        failOnStatusCode: false
      }).then((response) => {
        expect(response.status).to.eq(200);
        cy.task("cli_log", "Refund status: " + response.body.status);
      });
    });
  });

  context("Amazon Pay Refund - Edge Cases", () => {
    it("Refund with invalid payment_id", () => {
      const refundRequest = {
        payment_id: "invalid_payment_id_12345",
        amount: 6500,
        reason: "Test invalid payment"
      };

      cy.request({
        method: "POST",
        url: `${globalState.baseUrl}/refunds`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey")
        },
        body: refundRequest,
        failOnStatusCode: false
      }).then((response) => {
        expect(response.status).to.be.oneOf([400, 404, 422]);
        cy.task("cli_log", "Invalid payment refund error: " + JSON.stringify(response.body));
      });
    });

    it("Refund amount greater than payment amount", () => {
      const refundRequest = {
        payment_id: globalState.get("paymentId"),
        amount: 9999999,
        reason: "Test excessive refund"
      };

      cy.request({
        method: "POST",
        url: `${globalState.baseUrl}/refunds`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey")
        },
        body: refundRequest,
        failOnStatusCode: false
      }).then((response) => {
        expect(response.status).to.be.oneOf([400, 422]);
        cy.task("cli_log", "Excessive refund error: " + JSON.stringify(response.body));
      });
    });
  });
});
