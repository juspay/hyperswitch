import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Airwallex Alternative Payments", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");
        const airwallexLists = [
          CONNECTOR_LISTS.INCLUDE.AIRWALLEX_WALLET,
          CONNECTOR_LISTS.INCLUDE.AIRWALLEX_PAYLATER,
          CONNECTOR_LISTS.INCLUDE.AIRWALLEX_BANK_TRANSFER,
        ];
        const shouldRun = airwallexLists.some((list) =>
          list.includes(connector)
        );
        if (!shouldRun) {
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

  context("PayPal Wallet Redirect flow", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");
          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.AIRWALLEX_WALLET
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["AtomeAutoCapture"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-paypal-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalRedirect"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-wallet-redirection-call-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      expect(
        nextActionUrl,
        "nextActionUrl should be defined before handling wallet redirection"
      ).to.be.a("string");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("sync-payment-status-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalRedirect"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Skrill Wallet Redirect flow", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");
          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.AIRWALLEX_WALLET
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("Skrill");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-skrill-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Skrill"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-wallet-redirection-call-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      expect(
        nextActionUrl,
        "nextActionUrl should be defined before handling wallet redirection"
      ).to.be.a("string");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("sync-payment-status-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Skrill"];

      cy.wait(5000);
      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "succeeded",
      });
    });
  });

  context("Klarna PayLater Auto Capture flow", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");
          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.AIRWALLEX_PAYLATER
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-klarna-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Klarna"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-paylater-redirection-call-test", () => {
      const expected_redirection =
        globalState.get("baseUrl") + "/payments/completion";
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handlePayLaterRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("sync-payment-status-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Klarna"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Atome PayLater Auto Capture flow", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");
          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.AIRWALLEX_PAYLATER
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-atome-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Atome"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-paylater-redirection-call-test", () => {
      const expected_redirection =
        globalState.get("baseUrl") + "/payments/completion";
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handlePayLaterRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("sync-payment-status-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Atome"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Indonesian Bank Transfer flow", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");
          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.AIRWALLEX_BANK_TRANSFER
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("IndonesianBankTransfer");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-bank-transfer-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["IndonesianBankTransfer"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-bank-transfer-redirection-call-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("sync-payment-status-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["IndonesianBankTransfer"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });
});
