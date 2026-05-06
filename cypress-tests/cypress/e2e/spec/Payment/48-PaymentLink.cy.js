import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

// Payment Link is a CORE PLATFORM FEATURE - not connector specific
// This test verifies the Payment Link API functionality without relying on any specific connector

describe("Payment Link - Hosted payment link generation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Payment Link - Basic creation and retrieval", () => {
    it("Create Payment Intent with Payment Link -> Initiate Payment Link -> Retrieve Payment Link -> List Payment Links", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Payment Link", () => {
        const data = {
          Request: {
            currency: "USD",
            amount: 6000,
            description: "Test Payment Link",
            email: "test@example.com",
          },
          Response: {
            status: 200,
          },
        };

        cy.createPaymentIntentWithPaymentLinkTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      cy.step("Initiate Payment Link (Customer-Facing)", () => {
        const data = {};
        cy.initiatePaymentLinkTest(data, globalState);
      });

      cy.step("Retrieve Payment Link (Merchant API)", () => {
        const data = {};
        cy.retrievePaymentLinkTest(data, globalState);
      });

      cy.step("List Payment Links", () => {
        const data = {};
        cy.listPaymentLinksTest(data, globalState);
      });
    });
  });

  context("Payment Link - With Metadata", () => {
    it("Create Payment Intent with Payment Link and metadata -> Initiate Payment Link", () => {
      cy.step("Create Payment Intent with Payment Link and metadata", () => {
        const data = {
          Request: {
            currency: "USD",
            amount: 6500,
            description: "Test Payment Link with Metadata",
            email: "test@example.com",
            metadata: {
              order_id: "ORD-12345",
              customer_tier: "premium",
            },
          },
          Response: {
            status: 200,
          },
        };

        cy.createPaymentIntentWithPaymentLinkTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      cy.step("Initiate Payment Link (Customer-Facing)", () => {
        const data = {};
        cy.initiatePaymentLinkTest(data, globalState);
      });
    });
  });

  context("Payment Link - Edge Cases", () => {
    it("Create Payment Intent without Payment Link -> Should not have payment_link in response", () => {
      const profile_id = globalState.get("profileId") || globalState.get("defaultProfileId");
      
      const requestBody = {
        ...fixtures.createPaymentBody,
        currency: "USD",
        amount: 6000,
        description: "Test without Payment Link",
        email: "test@example.com",
        authentication_type: "no_three_ds",
        capture_method: "automatic",
        customer_id: globalState.get("customerId"),
        profile_id: profile_id,
        // Not setting payment_link: true
      };

      const headers = {
        "Content-Type": "application/json",
        Accept: "application/json",
        "api-key": globalState.get("apiKey"),
      };

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments`,
        headers,
        failOnStatusCode: false,
        body: requestBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        expect(response.body).to.have.property("client_secret");
        // Verify payment_link is null when not requested
        expect(response.body.payment_link).to.be.null;
      });
    });
  });
});
