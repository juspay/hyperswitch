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
        // `globalState` can only be accessed in the `it` block
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
        // `globalState` can only be accessed in the `it` block
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
});
