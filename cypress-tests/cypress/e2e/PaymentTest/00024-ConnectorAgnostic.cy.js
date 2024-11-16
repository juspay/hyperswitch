import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
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
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.createBusinessProfile,
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
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          configs,
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm Payment", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.confirmCallTest(
          configs,
          fixtures.confirmBody,
          req_data,
          res_data,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("List Payment Method for Customer using Client Secret", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.createBusinessProfile,
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
          fixtures.updateBusinessProfile,
          true,
          globalState
        );
      });

      it("Create Payment Intent", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          configs,
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("List Payment Method for Customer", () => {
        cy.listCustomerPMByClientSecret(globalState);
      });
    }
  );

  context("Connector Agnostic Enabled for Profile 1 and Profile 2", () => {
    let should_continue = true;

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(fixtures.createBusinessProfile, globalState);
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
        fixtures.updateBusinessProfile,
        true,
        globalState
      );
    });

    it("Create Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentOffSession"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        configs,
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Confirm Payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SaveCardUseNo3DSAutoCaptureOffSession"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmCallTest(
        configs,
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("List Payment Method for Customer using Client Secret", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(fixtures.createBusinessProfile, globalState);
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
        fixtures.updateBusinessProfile,
        true,
        globalState
      );
    });

    it("Create Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentOffSession"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        configs,
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("List Payment Method for Customer", () => {
      cy.listCustomerPMByClientSecret(globalState);
    });
  });
});
