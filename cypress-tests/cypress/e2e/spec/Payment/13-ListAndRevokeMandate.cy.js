import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - List and revoke Mandates flow test", () => {
  before(function () {
    // Changed to regular function instead of arrow function
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        // Skip running test against a connector that is added in the exclude list
        if (
          utils.shouldExcludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.EXCLUDE.MANDATE_ID_TEST
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          6000,
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
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthMandate"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        0,
        true,
        "automatic",
        "setup_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("list-mandate-call-test", () => {
      cy.listMandateCallTest(globalState);
    });

    it("Confirm No 3DS MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
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
