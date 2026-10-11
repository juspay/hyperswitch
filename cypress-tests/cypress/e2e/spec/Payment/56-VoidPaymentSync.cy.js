import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Payment sync after void flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.VOID_PAYMENT_SYNC
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

  context("Card - sync a voided payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Void Payment -> Retrieve Payment after Void", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Void Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Void", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment after Void");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: data.Response.body.status,
        });
      });
    });
  });
});
