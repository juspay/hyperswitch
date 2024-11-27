import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
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

        if (should_continue)
          should_continue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          data
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

        if (should_continue)
          should_continue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          data
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          data
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
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthPaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthConfirmPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve Payment Call Test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthConfirmPayment"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Retrieve CustomerPM Call Test", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Create Recurring Payment Intent", () => {
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

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Confirm Recurring Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardConfirmAutoCaptureOffSession"];

      cy.saveCardConfirmCallTest(
        fixtures.saveCardConfirmBody,
        data,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });
  });
});
