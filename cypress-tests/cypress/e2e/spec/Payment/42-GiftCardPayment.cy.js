import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
  should_continue_further,
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
        if (!should_continue_further(data)) {
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
          if (!should_continue_further(data)) {
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
          if (!should_continue_further(data)) {
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

  context("PaySafeCard gift card - redirect flow", () => {
    it("Create Payment Intent -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["PaymentIntent"]("PaySafeCard");

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!shouldContinue) return;
        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["PaySafeCardGiftCard"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (confirmData.Request && confirmData.Request.payment_method_type) {
          globalState.set(
            "paymentMethodType",
            confirmData.Request.payment_method_type
          );
        }

        if (!shouldContinue) return;
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["PaySafeCardGiftCard"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Givex gift card - refund flow", () => {
    it("Create and Confirm Givex Gift Card Payment -> Refund Payment -> Sync Refund Payment", () => {
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
        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "gift_card_pm"
        ]["GivexGiftCardRefund"];

        if (!should_continue_further(refundData)) {
          shouldContinue = false;
          return;
        }

        cy.refundCallTest(fixtures.refundBody, refundData, globalState);

        if (!shouldContinue) return;
        if (!should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["gift_card_pm"]["GivexGiftCardSyncRefund"];

        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
