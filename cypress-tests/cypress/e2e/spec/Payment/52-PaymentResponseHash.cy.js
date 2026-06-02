import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

function setup3DSPayment(gs, options = {}) {
  const { includeRedirection = true } = options;
  let shouldContinue = true;

  const data = getConnectorDetails(gs.get("connectorId"))["card_pm"][
    "PaymentIntent"
  ];

  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    data,
    "three_ds",
    "automatic",
    gs
  );

  if (!utils.should_continue_further(data)) {
    shouldContinue = false;
  }

  if (!shouldContinue) {
    cy.task("cli_log", "setup3DSPayment: stopping after createPaymentIntent");
    gs.set("_setup3DSContinue", false);
    return;
  }

  cy.paymentMethodsCallTest(gs);

  const confirmData = getConnectorDetails(gs.get("connectorId"))["card_pm"][
    "3DSAutoCapture"
  ];

  cy.confirmCallForHashTest(fixtures.confirmBody, confirmData, true, gs);

  if (!utils.should_continue_further(confirmData)) {
    shouldContinue = false;
  }

  if (shouldContinue) {
    cy.captureRedirectReturnUrl(gs);
  }

  if (includeRedirection && shouldContinue) {
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(gs, expected_redirection);
  }

  gs.set("_setup3DSContinue", shouldContinue);
}

describe("Card - Payment Response Hash flow test", () => {
  let shouldContinue = true;

  before("seed global state and check account config", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.PAYMENT_RESPONSE_HASH.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
      cy.fetchPaymentResponseHashConfig(globalState);
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("No3DS Auto-Capture - Verify Payment Response Hash Config", () => {
    it("create payment intent -> confirm payment -> verify payment response hash", () => {
      let stepContinue = true;

      cy.step("create payment intent", () => {
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
          stepContinue = false;
        }
      });

      cy.step("confirm payment", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: confirm payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallForHashTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );

        if (!utils.should_continue_further(data)) {
          stepContinue = false;
        }
      });

      cy.step("retrieve payment", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }

        cy.retrievePaymentCallTest({ globalState });
      });

      cy.step("verify payment response hash", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: verify payment response hash");
          return;
        }

        cy.assertPaymentResponseHashEnabled(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Verify Redirect Signature", () => {
    it("setup 3DS -> verify redirect signature", () => {
      setup3DSPayment(globalState, { includeRedirection: false });

      cy.step("verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify redirect signature");
          return;
        }

        cy.verifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Compute and Verify Redirect Signature", () => {
    it("setup 3DS -> compute HMAC and compare with redirect signature", () => {
      setup3DSPayment(globalState, { includeRedirection: false });

      cy.step("compute and verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task(
            "cli_log",
            "Skipping step: compute and verify redirect signature"
          );
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Failure Scenarios for Invalid Signatures", () => {
    it("setup 3DS -> compute HMAC -> verify tampered and wrong-key signatures fail", () => {
      setup3DSPayment(globalState, { includeRedirection: false });

      cy.step("compute and verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task(
            "cli_log",
            "Skipping step: compute and verify redirect signature"
          );
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });

      cy.step("verify tampered signature fails", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify tampered signature fails");
          return;
        }

        cy.verifyTamperedSignatureFails(globalState);
      });
    });
  });
});
