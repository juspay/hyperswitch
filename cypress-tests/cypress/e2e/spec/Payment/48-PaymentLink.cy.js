import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

/**
 * Payment Link Test Suite
 *
 * Feature: Generate hosted payment link for a payment
 * Endpoints:
 *   - POST /payments (payment_link=true) - Create payment with payment link
 *   - GET /payment_link/{id} - Retrieve payment link
 *
 * Configuration Source: business_profile.rs:payment_link_config
 *
 * This is a CORE PLATFORM FEATURE - not connector specific
 */

describe("Payment Link - Hosted payment link generation", () => {
  context("Payment Link - Basic creation and retrieval", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create Payment Intent with Payment Link", () => {
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

    it("Initiate Payment Link (Customer-Facing)", () => {
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Retrieve Payment Link (Merchant API)", () => {
      cy.retrievePaymentLinkTest({}, globalState);
    });

    it("List Payment Links", () => {
      cy.listPaymentLinksTest({}, globalState);
    });
  });

  context("Payment Link - With Metadata", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create Payment Intent with Payment Link and metadata", () => {
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

    it("Initiate Payment Link (Customer-Facing)", () => {
      cy.initiatePaymentLinkTest({}, globalState);
    });
  });

  context("Payment Link - Configuration Variations", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create Payment Link with custom theme color", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 7000,
          description: "Test with custom theme",
          email: "test@example.com",
          payment_link_config: {
            theme: "#FF6B35",
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

    it("Create Payment Link with merchant logo", () => {
      const data = {
        Request: {
          currency: "EUR",
          amount: 8000,
          description: "Test with merchant logo",
          email: "test@example.com",
          payment_link_config: {
            logo: "https://example.com/logo.png",
            seller_name: "Test Merchant Inc",
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

    it("Create Payment Link with accordion SDK layout", () => {
      const data = {
        Request: {
          currency: "GBP",
          amount: 5500,
          description: "Test with accordion layout",
          email: "test@example.com",
          payment_link_config: {
            sdk_layout: "accordion",
            display_sdk_only: false,
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

    it("Create Payment Link with display SDK only mode", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 4500,
          description: "Test SDK only mode",
          email: "test@example.com",
          payment_link_config: {
            display_sdk_only: true,
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

    it("Create Payment Link with transaction details", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 9900,
          description: "Test with transaction details",
          email: "test@example.com",
          payment_link_config: {
            transaction_details: [
              {
                name: "Product A",
                quantity: 2,
                unit_price: 4500,
              },
              {
                name: "Tax",
                quantity: 1,
                unit_price: 900,
              },
            ],
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

    it("Create Payment Link with all options combined", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 12000,
          description: "Test with all config options",
          email: "test@example.com",
          payment_link_config: {
            theme: "#4CAF50",
            logo: "https://example.com/full-logo.png",
            seller_name: "Premium Merchant",
            sdk_layout: "tabs",
            display_sdk_only: false,
            enabled_saved_payment_method: true,
            show_card_form_by_default: true,
            payment_button_text: "Pay Securely",
            background_colour: "#FFFFFF",
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
  });

  context("Payment Link - Edge Cases and Error Scenarios", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create Payment Intent without Payment Link - should not have payment_link in response", () => {
      const profile_id =
        globalState.get("profileId") || globalState.get("defaultProfileId");

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

    it("Retrieve non-existent Payment Link - should return 404", () => {
      const apiKey = globalState.get("apiKey");
      const baseUrl = globalState.get("baseUrl");
      const fakePaymentLinkId = "non_existent_link_12345";

      cy.request({
        method: "GET",
        url: `${baseUrl}/payment_link/${fakePaymentLinkId}`,
        headers: {
          "Content-Type": "application/json",
          "api-key": apiKey,
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(404);
      });
    });

    it("Create Payment Link with zero amount", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 0,
          description: "Test zero amount payment link",
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

    it("Create Payment Link with minimum amount (1 cent)", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 1,
          description: "Test minimum amount payment link",
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

    it("Create Payment Link with large amount", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 999999,
          description: "Test large amount payment link",
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
  });

  context("Payment Link - Business Profile Config", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Update business profile with payment_link_config (default theme)", () => {
      cy.updateBusinessProfilePaymentLinkConfigTest(
        {
          default_config: {
            theme: "#2D6CDF",
            logo: "https://example.com/merchant-logo.png",
            seller_name: "Profile Configured Merchant",
            sdk_layout: "accordion",
          },
        },
        globalState
      );
    });

    it("Create Payment Link after profile config update - should inherit theme", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 3000,
          description: "Test with inherited profile config",
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

    it("Override profile config with request-level payment_link_config", () => {
      const data = {
        Request: {
          currency: "USD",
          amount: 4000,
          description: "Test with override config",
          email: "test@example.com",
          payment_link_config: {
            theme: "#FF5722",
            seller_name: "Override Merchant",
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

    it("Reset business profile payment_link_config", () => {
      cy.updateBusinessProfilePaymentLinkConfigTest(
        {
          default_config: {
            theme: null,
            logo: null,
            seller_name: null,
            sdk_layout: null,
          },
        },
        globalState
      );
    });
  });

  context("Payment Link - Multiple Currencies", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    const currencies = [
      { currency: "USD", amount: 6000 },
      { currency: "EUR", amount: 5500 },
      { currency: "GBP", amount: 5000 },
      { currency: "INR", amount: 50000 },
      { currency: "AUD", amount: 8000 },
    ];

    currencies.forEach(({ currency, amount }) => {
      it(`Create Payment Link in ${currency}`, () => {
        const data = {
          Request: {
            currency: currency,
            amount: amount,
            description: `Test Payment Link in ${currency}`,
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
    });
  });
});
