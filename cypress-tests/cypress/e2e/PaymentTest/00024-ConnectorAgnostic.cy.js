import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { payment_methods_enabled } from "../PaymentUtils/Commons";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Connector Agnostic Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
  context(
    "Connector Agnostic Disabled for Profile 1 and Enabled for Profile 2",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("List Payment Method for Customer using Client Secret", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Enable Connector Agnostic for Business Profile", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("List Payment Method for Customer", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });
    }
  );

  context("Connector Agnostic Enabled for Profile 1 and Profile 2", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("connector-create-call-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });

    it("Create Customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Enable Connector Agnostic for Business Profile", () => {
      cy.UpdateBusinessProfileTest(
        fixtures.businessProfile.bpUpdate,
        true, // is_connector_agnostic_enabled
        false, // collect_billing_address_from_wallet_connector
        false, // collect_shipping_address_from_wallet_connector
        false, // always_collect_billing_address_from_wallet_connector
        false, // always_collect_shipping_address_from_wallet_connector
        globalState
      );
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer using Client Secret", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("connector-create-call-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });

    it("Enable Connector Agnostic for Business Profile", () => {
      cy.UpdateBusinessProfileTest(
        fixtures.businessProfile.bpUpdate,
        true, // is_connector_agnostic_enabled
        false, // collect_billing_address_from_wallet_connector
        false, // collect_shipping_address_from_wallet_connector
        false, // always_collect_billing_address_from_wallet_connector
        false, // always_collect_shipping_address_from_wallet_connector
        globalState
      );
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Payment Method for Customer", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });
  });
});
