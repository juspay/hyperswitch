import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("Payment Manual Update Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.MANUAL_PAYMENT_UPDATE
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

  context("Manual Payment Update - Happy Path", () => {
    it("Create Payment Intent -> Manual Update -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update Payment Attempt", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Manual Update Payment Attempt");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Manual Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Manual Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Configs: {
              skipBillingAssertion: true,
            },
          },
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Status Only", () => {
    it("Create Payment Intent -> Manual Update Status Only -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update Payment Status Only", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment Status Only"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateStatusOnly"];

        cy.manualPaymentStatusUpdateTest(globalState, data);
      });

      cy.step("Retrieve Payment to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Status Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Configs: {
              skipBillingAssertion: true,
            },
          },
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Amount Captured", () => {
    it("Create Payment Intent -> Manual Update with amount_captured -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update Payment with amount_captured", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment with amount_captured"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCaptured"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify amount_captured Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify amount_captured Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCaptured"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Negative Cases", () => {
    it("Create Payment Intent -> Manual Update with Invalid Attempt ID", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update with Invalid Attempt ID", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with Invalid Attempt ID"
          );
          return;
        }

        cy.manualPaymentUpdateNegativeTest(globalState, "invalid_attempt_id");
      });
    });
  });

  context("Manual Payment Update - Amount Captured Exceeds Amount", () => {
    it("Create Payment Intent -> Manual Update with amount_captured exceeding payment amount", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step(
        "Manual Update with amount_captured exceeding payment amount",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Manual Update with amount_captured exceeding payment amount"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ManualPaymentUpdateAmountCapturedExceedsAmount"];

          cy.manualPaymentStatusUpdateTest(globalState, data);
        }
      );
    });
  });

  context("Manual Payment Update - Amount Captured Boundary", () => {
    it("Create Payment Intent -> Manual Update with amount_captured equal to payment amount -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step(
        "Manual Update Payment with amount_captured equal to payment amount",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Manual Update Payment with amount_captured equal to payment amount"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ManualPaymentUpdateAmountCapturedBoundary"];

          cy.manualPaymentStatusUpdateTest(globalState, data);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Retrieve Payment to Verify amount_captured Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify amount_captured Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedBoundary"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Amount Captured Zero", () => {
    it("Create Payment Intent -> Manual Update with zero amount_captured -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update Payment with zero amount_captured", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment with zero amount_captured"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedZero"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify amount_captured Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify amount_captured Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedZero"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Amount Captured Negative", () => {
    it("Create Payment Intent -> Manual Update with negative amount_captured -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update Payment with negative amount_captured", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment with negative amount_captured"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedNegative"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify amount_captured Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify amount_captured Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedNegative"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Amount Captured Without Status", () => {
    it("Create Payment Intent -> Manual Update with amount_captured without attempt_status -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step(
        "Manual Update Payment with amount_captured without attempt_status",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Manual Update Payment with amount_captured without attempt_status"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ManualPaymentUpdateAmountCapturedWithoutStatus"];

          cy.manualPaymentStatusUpdateTest(globalState, data);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Retrieve Payment to Verify amount_captured Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify amount_captured Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedWithoutStatus"];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Manual Payment Update - Amount Captured Invalid Type", () => {
    it("Create Payment Intent -> Manual Update with invalid amount_captured type", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update with invalid amount_captured type", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with invalid amount_captured type"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateAmountCapturedInvalidType"];

        cy.manualPaymentStatusUpdateTest(globalState, data);
      });
    });
  });

  context("Manual Payment Update - Edge Cases", () => {
    it("Create Payment Intent -> Manual Update with Custom Error -> Verify Persistence", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
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

      cy.step("Manual Update with Custom Error Code and Message", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with Custom Error Code and Message"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Persistence"
          );
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Configs: {
              skipBillingAssertion: true,
            },
          },
          unconfirmedPayment: true,
        });
      });
    });
  });
});
