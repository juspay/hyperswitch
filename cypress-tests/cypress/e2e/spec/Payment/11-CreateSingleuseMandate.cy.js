import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SingleUse Mandates flow test", () => {
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
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
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
        ]["MandateSingleUseNo3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("cit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("list-mandate-call-test", () => {
        cy.listMandateCallTest(globalState);
      });
    }
  );

  context(
    "Card - No threeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("cit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

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
    }
  );

  context(
    "Card - Recurring mandate with greater mandate amount flow test",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("cit-capture-call-test", () => {
        if (!shouldContinue) {
          this.skip();
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Retrieve payment", () => {
        if (!shouldContinue) {
          this.skip();
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      it("Confirm No 3DS MIT", () => {
        if (!shouldContinue) {
          this.skip();
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          60000,
          true,
          "manual",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Zero Auth Mandate Create + Confirm Automatic CIT and Single use MIT payment flow test",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ZeroAuthMandate"];

        cy.retrievePaymentCallTest({ globalState, data });
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

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );
});
