import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;

describe("Core flows", () => {
  context("Merchant core flows", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("merchant create call", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("merchant retrieve call", () => {
      cy.merchantRetrieveCall(globalState);
    });

    it("merchant list call", () => {
      cy.merchantListCall(globalState);
    });

    it("merchant update call", () => {
      cy.merchantUpdateCall(fixtures.merchantUpdateBody, globalState);
    });
  });

  context("API key core flows", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("API key create call", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("API key update call", () => {
      cy.apiKeyUpdateCall(fixtures.apiKeyUpdateBody, globalState);
    });

    it("API key retrieve call", () => {
      cy.apiKeyRetrieveCall(globalState);
    });

    it("API key list call", () => {
      cy.apiKeyListCall(globalState);
    });
  });

  context("Customer core flows", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Customer create call", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Customer list call", () => {
      cy.customerListCall(globalState);
    });

    it("Customer retrieve call", () => {
      cy.customerRetrieveCall(globalState);
    });

    it("Customer update call", () => {
      cy.customerUpdateCall(fixtures.customerUpdateBody, globalState);
    });

    it("Ephemeral key generate call", () => {
      cy.ephemeralGenerateCall(globalState);
    });
  });

  context("Merchant Connector Account core flows", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    context("Create Multiple Merchant Connector Accounts", () => {
      it("1st Connector create call", () => {
        const connector_id = globalState.data.connectorId;
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          connector_id,
          "first_" + connector_id
        );
      });

      it("2nd Connector create call", () => {
        const connector_id = globalState.data.connectorId;
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          connector_id,
          "second_" + connector_id
        );
      });
    });

    it("Connector create call", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });

    it("Connector retrieve call", () => {
      cy.connectorRetrieveCall(globalState);
    });

    it("Connector update call", () => {
      cy.connectorUpdateCall(
        "payment_processor",
        fixtures.updateConnectorBody,
        globalState
      );
    });

    it("List connectors by MID", () => {
      cy.connectorListByMid(globalState);
    });
  });

  context("Client Secret Session Expiry", () => {
    const SESSION_EXPIRY_WAIT = 65000;
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Update business profile with session_expiry", () => {
      const updateBusinessProfileBody = {
        session_expiry: 60,
      };
      cy.UpdateBusinessProfileTest(
        updateBusinessProfileBody,
        false,
        false,
        false,
        false,
        false,
        globalState
      );
    });

    it("Create payment - session expiry inherited from business profile", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm payment after session expiry - should fail with ClientSecretExpired", () => {
      // eslint-disable-next-line cypress/no-unnecessary-waiting
      cy.wait(SESSION_EXPIRY_WAIT);
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SessionExpiredConfirmPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Create payment with session_expiry in request body", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithSessionExpiry"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  // Payment Link is a core platform feature — not connector specific
  // Endpoint: POST /payments (payment_link=true) + GET /payment_link/{id}
  // Source: business_profile.rs:payment_link_config
  context("Payment Link - Basic creation and retrieval", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
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

  context("Payment Link - Configuration Variations", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
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
  });

  context("Payment Link - Edge Cases", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
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
      };

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments`,
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
        body: requestBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        expect(response.body.payment_link).to.be.null;
      });
    });

    it("Retrieve non-existent Payment Link - should return 404", () => {
      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/payment_link/non_existent_link_12345`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(404);
      });
    });
  });

  context("List Connector Feature Matrix", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("List connector feature matrix call", () => {
      cy.ListConnectorsFeatureMatrixCall(globalState);
    });
  });

  context("Delete calls", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Customer delete call", () => {
      cy.customerDeleteCall(globalState);
    });

    it("API key delete call", () => {
      cy.apiKeyDeleteCall(globalState);
    });

    it("Connector delete call", () => {
      cy.connectorDeleteCall(globalState);
    });

    it("Merchant delete call", () => {
      cy.merchantDeleteCall(globalState);
    });
  });
});
