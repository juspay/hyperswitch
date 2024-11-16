import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - MultiUse Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
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
        ]["MandateMultiUseNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          configs,
          fixtures.citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
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
        ]["MandateMultiUseNo3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          configs,
          fixtures.citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          configs,
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT 1", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          configs,
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT 2", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          configs,
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow test",
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
        ]["MandateMultiUseNo3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          configs,
          fixtures.citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        
        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          configs,
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
  
      if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          6500,
          true,
          "automatic",
          globalState
        );
      });
    }
  );
});
