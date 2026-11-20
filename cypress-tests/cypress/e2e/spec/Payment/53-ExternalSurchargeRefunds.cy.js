import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  captureFullAmount,
  cardData,
  refundShareOfOrderAmount,
  runSurchargePayment,
  shouldSkipExternalSurcharge,
} from "../../../utils/externalSurcharge";

let globalState;

// Surcharge refunds (PR #15155). A partial refund returns the order amount plus
// a share of the surcharge, calculated by the surcharge connector (InterPayments),
// so the amounts are checked for consistency, not for fixed values.
//
// Needs the surcharge connector that 03-ConnectorCreate creates from the
// "interpayments" entry in creds.json; the spec skips itself without it.
describe("External surcharge - refunds", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        return shouldSkipExternalSurcharge(globalState);
      })
      .then((shouldSkip) => {
        skip = shouldSkip;
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("unlink the surcharge connector and flush global state", () => {
    cy.unlinkSurchargeConnectorFromProfile(globalState);
    cy.task("setGlobalState", globalState.data);
  });

  it("Point the profile at the surcharge connector, create a customer", () => {
    cy.linkSurchargeConnectorToProfile(globalState);
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  context("Partial refunds on an automatic capture payment", () => {
    it("Payment -> invalid refunds -> partial refunds -> rest of the order amount -> refund after full refund", () => {
      const state = runSurchargePayment(globalState);
      const refundStep = (name, run) =>
        cy.step(name, () => {
          if (!state.shouldContinue) {
            cy.task("cli_log", `Skipping step: ${name}`);
            return;
          }
          run();
        });

      refundStep("Refund of 0 is rejected", () =>
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          cardData(globalState, "ExternalSurchargeRefundZeroAmount"),
          globalState,
          "unusedRefund"
        )
      );

      refundStep("Negative refund is rejected", () =>
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          cardData(globalState, "ExternalSurchargeRefundNegativeAmount"),
          globalState,
          "unusedRefund"
        )
      );

      refundStep("Refund above the order amount is rejected", () =>
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          {
            ...cardData(globalState, "ExternalSurchargeRefundOverOrderAmount"),
            Request: { amount: globalState.get("paymentAmount") + 1 },
          },
          globalState,
          "unusedRefund"
        )
      );

      refundStep("First partial refund carries a share of the surcharge", () =>
        refundShareOfOrderAmount(globalState, 0.4, "firstSurchargeRefund")
      );

      refundStep("Retrieve the first refund: same amounts", () =>
        cy.surchargeRefundRetrieveTest("firstSurchargeRefund", globalState)
      );

      refundStep(
        "Second partial refund keeps the surcharge within what was charged",
        () =>
          refundShareOfOrderAmount(globalState, 0.3, "secondSurchargeRefund")
      );

      refundStep(
        "Refunding the rest of the order amount refunds the whole surcharge",
        () =>
          cy.surchargeRefundCallTest(
            fixtures.refundBody,
            {
              ...cardData(globalState, "ExternalSurchargeRefund"),
              Request: {
                amount:
                  globalState.get("paymentAmount") -
                  globalState.get("orderAmountRefunded"),
              },
            },
            globalState,
            "finalSurchargeRefund"
          )
      );

      refundStep("Refund after the whole order amount is refunded", () =>
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          {
            ...cardData(globalState, "ExternalSurchargeRefundExceedsCaptured"),
            Request: { amount: 1 },
          },
          globalState,
          "unusedRefund"
        )
      );
    });
  });

  context("Refund with no amount", () => {
    it("Payment -> refund with no amount -> second refund", () => {
      const state = runSurchargePayment(globalState);

      cy.step("Refund with no amount refunds the whole charged total", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund with no amount");
          return;
        }
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          cardData(globalState, "ExternalSurchargeRefundWholeOrder"),
          globalState,
          "wholeOrderSurchargeRefund"
        );
      });

      cy.step("A refund after the whole charged total is refunded", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund after full refund");
          return;
        }
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          {
            ...cardData(globalState, "ExternalSurchargeRefundExceedsCaptured"),
            Request: { amount: 1 },
          },
          globalState,
          "unusedRefund"
        );
      });
    });
  });

  context("Manual capture", () => {
    it("Payment -> refund before capture -> capture -> partial refund", () => {
      const state = runSurchargePayment(globalState, {
        captureMethod: "manual",
        confirmResponse: { status: 200, body: { status: "requires_capture" } },
      });

      cy.step("Refund before capture is rejected", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund before capture");
          return;
        }
        cy.surchargeRefundCallTest(
          fixtures.refundBody,
          {
            ...cardData(globalState, "ExternalSurchargeRefundBeforeCapture"),
            Request: {
              amount: Math.floor(globalState.get("paymentAmount") * 0.4),
            },
          },
          globalState,
          "unusedRefund"
        );
      });

      cy.step("Capture the full charged amount", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture");
          return;
        }
        captureFullAmount(globalState);
      });

      cy.step(
        "Partial refund after capture carries a share of the surcharge",
        () => {
          if (!state.shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund after capture");
            return;
          }
          refundShareOfOrderAmount(globalState, 0.4, "manualSurchargeRefund");
        }
      );
    });
  });
});
