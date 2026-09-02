import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";
import { isMockServer } from "../../../support/mitmProxy";

let globalState;

describe("Client Session Validation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  before("connector gate", function () {
    const connectorId = globalState.get("connectorId");
    if (
      !CONNECTOR_LISTS.INCLUDE.CLIENT_SESSION_VALIDATION.includes(connectorId)
    ) {
      this.skip();
    }
  });

  context("Valid SDK Authorization - Confirm with SDK Authorization", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
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

    it("Confirm with valid SDK Authorization", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);
    });

    it("Retrieve Payment", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data: confirmData });
    });
  });

  context(
    "Invalid SDK Authorization - Confirm with tampered sdk_authorization",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
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

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm with invalid sdk_authorization - expect 401", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionInvalidConfirm"];

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState,
          "invalid_session"
        );
      });
    }
  );

  context(
    "Missing SDK Authorization - Confirm without sdk_authorization (legacy fallback)",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
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

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm without sdk_authorization - legacy fallback", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState,
          "missing_session"
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(confirmData);
      });

      it("Retrieve Payment", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    }
  );

  context(
    "Expired SDK Authorization - Confirm after session TTL expires",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent with 60s session expiry", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const createBody = {
          ...fixtures.createPaymentBody,
          session_expiry: 60,
        };

        cy.createPaymentIntentTest(
          createBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Wait for Redis session TTL to expire", () => {
        // eslint-disable-next-line cypress/no-unnecessary-waiting
        if (!isMockServer()) cy.wait(61000); // Wait 61 seconds for Redis TTL to expire
      });

      it("Confirm with expired sdk_authorization - expect 401", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionInvalidConfirm"];

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    }
  );
});
