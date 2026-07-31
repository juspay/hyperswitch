import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Refund Type (instant/scheduled) flow - No 3DS", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.REFUND_TYPE.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("Card - Instant Refund Type flow test for No-3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund with Instant Type -> Sync Refund Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund with Instant Type", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund with Instant Type");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundInstant"];
        const newRefundData = {
          ...refundData,
          Response: refundData.ResponseCustom || refundData.Response,
        };
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
        if (!utils.should_continue_further(newRefundData)) {
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
        )["card_pm"]["SyncRefund"];
        const newData = {
          ...syncRefundData,
          Response: syncRefundData.ResponseCustom || syncRefundData.Response,
        };
        cy.syncRefundCallTest(newData, globalState);
      });
    });
  });

  context("Card - Scheduled Refund Type flow test for No-3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund with Scheduled Type -> Sync Refund Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund with Scheduled Type", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund with Scheduled Type");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundScheduled"];
        const newRefundData = {
          ...refundData,
          Response: refundData.ResponseCustom || refundData.Response,
        };
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
        if (!utils.should_continue_further(newRefundData)) {
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
        )["card_pm"]["SyncRefundScheduled"];
        const newData = {
          ...syncRefundData,
          Response: syncRefundData.ResponseCustom || syncRefundData.Response,
        };
        cy.syncRefundCallTest(newData, globalState);
      });
    });
  });

  context(
    "Card - Instant Refund Type flow test Create+Confirm for No-3DS",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Refund with Instant Type -> Sync Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment", () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund with Instant Type", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund with Instant Type");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["RefundInstant"];
          const newRefundData = {
            ...refundData,
            Response: refundData.ResponseCustom || refundData.Response,
          };
          cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
          if (!utils.should_continue_further(newRefundData)) {
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
          )["card_pm"]["SyncRefund"];
          const newData = {
            ...syncRefundData,
            Response: syncRefundData.ResponseCustom || syncRefundData.Response,
          };
          cy.syncRefundCallTest(newData, globalState);
        });
      });
    }
  );

  context(
    "Card - Scheduled Refund Type flow test Create+Confirm for No-3DS",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Refund with Scheduled Type -> Sync Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment", () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund with Scheduled Type", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund with Scheduled Type");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["RefundScheduled"];
          const newRefundData = {
            ...refundData,
            Response: refundData.ResponseCustom || refundData.Response,
          };
          cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
          if (!utils.should_continue_further(newRefundData)) {
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
          )["card_pm"]["SyncRefundScheduled"];
          const newData = {
            ...syncRefundData,
            Response: syncRefundData.ResponseCustom || syncRefundData.Response,
          };
          cy.syncRefundCallTest(newData, globalState);
        });
      });
    }
  );
});
