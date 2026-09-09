import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - CVC Omit and Risk Messages flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.CVC_OMIT
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("CVC omission via mandate MIT flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("cit-mandate-setup-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CvcOmitAndRiskMessages"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        100,
        true,
        "automatic",
        "setup_mandate",
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("mit-no-cvc-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        200,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("retrieve-mit-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "succeeded",
      });
    });
  });

  context("baseline payment with valid CVC", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-confirm-payment-with-cvc-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "succeeded",
      });
    });
  });

  context("payment with missing billing fields", () => {
    it("payment-with-missing-billing-fields-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CvcOmitNoBilling"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("negative - payment with empty CVC", () => {
    it("payment-with-empty-cvc-error-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CvcOmitEmptyCvc"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("negative - payment with omitted CVC field", () => {
    it("payment-with-omitted-cvc-error-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CvcOmitOmittedCvc"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });
});
