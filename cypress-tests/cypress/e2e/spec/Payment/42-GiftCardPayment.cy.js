import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Gift Card Payment - Adyen Givex", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(connector, CONNECTOR_LISTS.INCLUDE.GIFT_CARD)
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          cy.log(
            `Skipping gift card tests for connector: ${connector} — not in GIFT_CARD inclusion list`
          );
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Givex gift card - successful balance check", () => {
    it("Create and Confirm Givex Gift Card Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Givex Gift Card Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["GivexGiftCard"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (data && data.Response && data.Response.status === 501) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["GivexGiftCard"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Givex gift card - insufficient balance", () => {
    it("Create and Confirm Givex Gift Card Payment with insufficient balance -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step(
        "Create and Confirm Givex Gift Card Payment (Insufficient Balance)",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "gift_card_pm"
          ]["GivexGiftCardInsufficientBalance"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!shouldContinue) return;
          if (data && data.Response && data.Response.status === 501) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["GivexGiftCardInsufficientBalance"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Givex gift card - currency mismatch", () => {
    it("Create and Confirm Givex Gift Card Payment with currency mismatch -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step(
        "Create and Confirm Givex Gift Card Payment (Currency Mismatch)",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "gift_card_pm"
          ]["GivexGiftCardCurrencyMismatch"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!shouldContinue) return;
          if (data && data.Response && data.Response.status === 501) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["GivexGiftCardCurrencyMismatch"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
