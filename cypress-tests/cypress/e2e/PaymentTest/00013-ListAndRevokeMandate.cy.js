import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - List and revoke Mandates flow test", () => {
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
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
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
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          globalState
        );
      });

      it("list-mandate-call-test", () => {
        cy.listMandateCallTest(globalState);
      });

      it("revoke-mandate-call-test", () => {
        cy.revokeMandateCallTest(globalState);
      });

      it("revoke-revoked-mandate-call-test", () => {
        cy.revokeMandateCallTest(globalState);
      });
    }
  );
  context("Card - Zero auth CIT and MIT payment flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Confirm No 3DS CIT", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ZeroAuthMandate"
      ];
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
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("list-mandate-call-test", () => {
      cy.listMandateCallTest(globalState);
    });

    it("Confirm No 3DS MIT", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MITAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        req_data,
        res_data,
        7000,
        true,
        "automatic",
        globalState
      );
    });

    it("list-mandate-call-test", () => {
      cy.listMandateCallTest(globalState);
    });

    it("revoke-mandate-call-test", () => {
      cy.revokeMandateCallTest(globalState);
    });
  });
});
