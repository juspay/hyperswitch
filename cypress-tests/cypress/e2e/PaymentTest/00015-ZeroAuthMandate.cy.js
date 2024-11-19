import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - SingleUse Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and Single use MIT payment flow test",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          0,
          true,
          "automatic",
          "setup_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        let configs = validateConfig(data["Configs"]);

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );
  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and Multi use MIT payment flow test",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          0,
          true,
          "automatic",
          "setup_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        let configs = validateConfig(data["Configs"]);

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        let configs = validateConfig(data["Configs"]);

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );

  context("Card - Zero Auth Payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create No 3DS Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ZeroAuthPaymentIntent"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Confirm No 3DS payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ZeroAuthConfirmPayment"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Retrieve Payment Call Test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ZeroAuthConfirmPayment"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Retrieve CustomerPM Call Test", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Create Recurring Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentOffSession"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Confirm Recurring Payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SaveCardConfirmAutoCaptureOffSession"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.saveCardConfirmCallTest(
        fixtures.saveCardConfirmBody,
        req_data,
        res_data,
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });
  });
});
