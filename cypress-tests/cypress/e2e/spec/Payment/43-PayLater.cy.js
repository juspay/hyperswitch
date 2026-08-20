import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

// PayJustNow redirect-dependent tests are skipped because the PayJustNow sandbox
// returns 403 Forbidden on confirm-payment when the server's return_url is a
// localhost HTTP URL.
//
// Tracking issue: PAYA-1 — "Add PayJustNow PayLater connector"
// Re-enable these tests once the sandbox supports public HTTPS redirect URLs.
//
// Root cause:
//   The PayJustNow sandbox validates the return_url passed in the confirm request.
//   When the Hyperswitch server runs locally (base_url = http://localhost:8080),
//   the return_url is http://localhost:8080/payments/completion, which the sandbox
//   rejects with 403 Forbidden. The sandbox requires a publicly accessible HTTPS URL.
//
// Expected URL format:
//   https://*.ngrok-free.app/payments/completion
//   (or any public HTTPS endpoint that forwards to the local server on port 8080)
//
// Infrastructure fix plan:
//   1. Start an ngrok tunnel (or similar) targeting localhost:8080:
//        ngrok http 8080
//   2. Update development.toml [multitenancy.tenants.public] base_url to the
//      ngrok HTTPS URL so the server constructs a public return_url.
//   3. Restart the Hyperswitch server.
//   4. Remove the this.skip() calls in the three "PayJustNow PayLater" context
//      before-hooks below to re-enable the tests.
//
// Sandbox environment:
//   - Connector: payjustnow (BodyKey auth: api_key = merchant_account_id,
//     key1 = signing_key)
//   - Sandbox: PayJustNow test environment
//   - The sandbox returns 403 on localhost redirect URLs (requires public HTTPS)
//   - Currency: ZAR, Country: ZA
//   - Redirect flow: confirm → PayJustNow checkout page →
//     enter email customer@payjustnow.co.za + password "password" →
//     click "Complete Payment" → redirect back to return_url
//
// The test code is preserved below for easy re-enablement.

describe("PayLater tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAY_LATER
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

  context("Klarna PayLater - Auto Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Klarna PayLater - Manual Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ManualCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Klarna PayLater - Manual Capture with Capture and Retrieve flow test",
    () => {
      it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "pay_later_pm"
          ]["ManualCapture"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "manual",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Merchant Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["Klarna"];
          cy.confirmBankRedirectCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle PayLater Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
            return;
          }
          const expected_redirection =
            globalState.get("baseUrl") + "/payments/completion";
          const payment_method_type = globalState.get("paymentMethodType");
          cy.handlePayLaterRedirection(
            globalState,
            payment_method_type,
            expected_redirection
          );
        });

        cy.step("Capture Payment on wrong status", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Capture Payment on wrong status"
            );
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["CaptureOnWrongStatus"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        });

        cy.step("Retrieve Payment after failed Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after failed Capture"
            );
            return;
          }
          const klarnaData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["Klarna"];
          cy.retrievePaymentCallTest({ globalState, data: klarnaData });
        });
      });
    }
  );

  context("Klarna PayLater - Separate Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Klarna PayLater - Mandate AutoCapture flow test (CIT only)", () => {
    before(function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.PAY_LATER_KLARNA_MANDATE
        )
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm with Customer Acceptance (off_session) -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with Customer Acceptance (off_session)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["KlarnaMandateAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["KlarnaMandateAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Capture on wrong status - Error test", () => {
    it("Create Payment Intent -> Confirm Payment -> Attempt Capture on requires_customer_action status", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ManualCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Attempt Capture on wrong status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Attempt Capture on wrong status");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["CaptureOnWrongStatus"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
      });
    });
  });

  context("Confirm without payment_method_data - Error test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment without payment_method_data", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment without payment_method_data", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment without payment_method_data"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ConfirmWithoutPmData"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("Atome PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Atome", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.ATOME
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AtomeAutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Atome"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Atome"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("AfterpayClearpay PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support AfterpayClearpay", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFTERPAY_CLEARPAY
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpayAutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpay"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpay"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Alma PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Alma", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.ALMA
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AlmaAutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Alma"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Alma"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Walley PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Walley", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.WALLEY
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["WalleyAutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Walley"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Walley"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayJustNow PayLater - Create and Confirm flow test", () => {
    before("skip PayJustNow redirect-dependent tests", function () {
      // Skip reason: PayJustNow sandbox returns 403 Forbidden when the server's
      // return_url is a localhost HTTP URL (http://localhost:8080/payments/completion).
      // The sandbox validates the return_url and requires a publicly accessible HTTPS URL.
      //
      // Expected URL format: https://*.ngrok-free.app/payments/completion
      //   (or any public HTTPS endpoint forwarding to local server port 8080)
      //
      // Planned infra fix:
      //   1. Start ngrok tunnel: `ngrok http 8080`
      //   2. Update development.toml [multitenancy.tenants.public] base_url to
      //      the ngrok HTTPS URL so the server constructs a public return_url
      //   3. Restart Hyperswitch server
      //   4. Remove this this.skip() call to re-enable the test
      //
      // Sandbox env: payjustnow connector, BodyKey auth (api_key=merchant_account_id,
      //   key1=signing_key), currency ZAR, country ZA
      //
      // Tracking issue: PAYA-1 — re-enable when sandbox supports public HTTPS redirects
      this.skip();
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnow"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayJustNow PayLater - Full Refund flow test", () => {
    before("skip PayJustNow redirect-dependent tests", function () {
      // Skip reason: PayJustNow sandbox returns 403 Forbidden when the server's
      // return_url is a localhost HTTP URL (http://localhost:8080/payments/completion).
      // The sandbox validates the return_url and requires a publicly accessible HTTPS URL.
      //
      // Expected URL format: https://*.ngrok-free.app/payments/completion
      //   (or any public HTTPS endpoint forwarding to local server port 8080)
      //
      // Planned infra fix:
      //   1. Start ngrok tunnel: `ngrok http 8080`
      //   2. Update development.toml [multitenancy.tenants.public] base_url to
      //      the ngrok HTTPS URL so the server constructs a public return_url
      //   3. Restart Hyperswitch server
      //   4. Remove this this.skip() call to re-enable the test
      //
      // Sandbox env: payjustnow connector, BodyKey auth (api_key=merchant_account_id,
      //   key1=signing_key), currency ZAR, country ZA
      //
      // Tracking issue: PAYA-1 — re-enable when sandbox supports public HTTPS redirects
      this.skip();
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnow"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("PayJustNow PayLater - Partial Refund flow test", () => {
    before("skip PayJustNow redirect-dependent tests", function () {
      // Skip reason: PayJustNow sandbox returns 403 Forbidden when the server's
      // return_url is a localhost HTTP URL (http://localhost:8080/payments/completion).
      // The sandbox validates the return_url and requires a publicly accessible HTTPS URL.
      //
      // Expected URL format: https://*.ngrok-free.app/payments/completion
      //   (or any public HTTPS endpoint forwarding to local server port 8080)
      //
      // Planned infra fix:
      //   1. Start ngrok tunnel: `ngrok http 8080`
      //   2. Update development.toml [multitenancy.tenants.public] base_url to
      //      the ngrok HTTPS URL so the server constructs a public return_url
      //   3. Restart Hyperswitch server
      //   4. Remove this this.skip() call to re-enable the test
      //
      // Sandbox env: payjustnow connector, BodyKey auth (api_key=merchant_account_id,
      //   key1=signing_key), currency ZAR, country ZA
      //
      // Tracking issue: PAYA-1 — re-enable when sandbox supports public HTTPS redirects
      this.skip();
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Partial Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnow"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("PayJustNow In-Store PayLater - Create and Confirm flow test", () => {
    before(
      "skip if connector does not support PayJustNow In-Store",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAYJUSTNOWINSTORE
          )
        ) {
          this.skip();
        }
      }
    );

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnowinstore");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayJustNow In-Store PayLater - Full Refund flow test", () => {
    before(
      "skip if connector does not support PayJustNow In-Store",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAYJUSTNOWINSTORE
          )
        ) {
          this.skip();
        }
      }
    );

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnowinstore");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("PayJustNow In-Store PayLater - Partial Refund flow test", () => {
    before(
      "skip if connector does not support PayJustNow In-Store",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAYJUSTNOWINSTORE
          )
        ) {
          this.skip();
        }
      }
    );

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Partial Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnowinstore");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
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
          "pay_later_pm"
        ]["Payjustnowinstore"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
