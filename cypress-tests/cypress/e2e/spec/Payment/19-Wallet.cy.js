import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;
describe("Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Bluecode Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.BLUECODE_WALLET
            )
          ) {
            skip = true;
            return;
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
      ]["PaymentIntent"]("Bluecode");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm wallet redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Bluecode"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Bluecode"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("AliPayHk Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.ALIPAY_HK_WALLET
            )
          ) {
            skip = true;
            return;
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
      ]["PaymentIntent"]("AliPayHk");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm wallet redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AliPayHk"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AliPayHk"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "succeeded",
      });
    });
  });

  context("Mifinity Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.MIFINITY_WALLET
            )
          ) {
            skip = true;
            return;
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

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Wallet Redirection -> Retrieve Payment", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntent"]("Mifinity");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
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
          "wallet_pm"
        ]["Mifinity"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Wallet Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Wallet Redirection");
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
          "wallet_pm"
        ]["Mifinity"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayPal Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.PAYPAL_WALLET
            )
          ) {
            skip = true;
            return;
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("PaypalRedirect");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm PayPal redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalRedirect"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const nextActionUrl = globalState.get("nextActionUrl");
      expect(nextActionUrl, "PayPal redirect URL should be present").to.be.a(
        "string"
      ).and.not.be.empty;
    });

    it("Sync payment status", () => {
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

  context("PayPal Mandate CIT flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.PAYPAL_WALLET
            ) ||
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.PAYPAL_MANDATE
            )
          ) {
            skip = true;
            return;
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("PaypalRedirect");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm PayPal mandate CIT redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalRedirectMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const nextActionUrl = globalState.get("nextActionUrl");
      expect(
        nextActionUrl,
        "PayPal mandate redirect URL should be present"
      ).to.be.a("string").and.not.be.empty;
    });

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalRedirectMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("AliPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.ALIPAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Wallet Redirection -> Retrieve Payment", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntent"]("AliPay");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
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
          "wallet_pm"
        ]["AliPay"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Wallet Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Wallet Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleWalletRedirection(
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
          "wallet_pm"
        ]["AliPay"];
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "requires_customer_action",
        });
      });
    });
  });

  context("WeChatPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.WECHATPAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Wallet Redirection -> Retrieve Payment", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntent"]("WeChatPay");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
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
          "wallet_pm"
        ]["WeChatPay"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Wallet Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Wallet Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleWalletRedirection(
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
          "wallet_pm"
        ]["WeChatPay"];
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "requires_customer_action",
        });
      });
    });
  });

  context("MbWay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.MBWAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Wallet Redirection -> Retrieve Payment", () => {
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntent"]("MbWay");
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
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
          "wallet_pm"
        ]["MbWay"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Wallet Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Wallet Redirection");
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
          "wallet_pm"
        ]["MbWay"];
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: "requires_customer_action",
        });
      });
    });
  });

  context("Skrill - Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.SKRILL_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Skrill wallet tests — connector not in SKRILL_WALLET list"
            );
            this.skip();
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

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("Skrill");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Skrill wallet redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Skrill"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle Skrill bank redirect redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Skrill"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
});
describe("Stripe AliPay Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Stripe AliPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.ALIPAY_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Stripe AliPay wallet tests — connector not in ALIPAY_WALLET list"
            );
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
      ]["PaymentIntent"]("AliPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm AliPay redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AliPay"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = "ali_pay";
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AliPay"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe AliPay - Invalid Currency error test", () => {
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
              CONNECTOR_LISTS.INCLUDE.ALIPAY_WALLET
            )
          ) {
            skip = true;
            return;
          }
          const details = getConnectorDetails(connector);
          if (!details?.wallet_pm?.AliPayInvalidCurrency) {
            skip = true;
            return;
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

    it("confirm-alipay-with-invalid-currency-should-fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("AliPay", "INR");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm AliPay with invalid currency", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AliPayInvalidCurrency"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});

describe("Stripe AmazonPay Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Stripe AmazonPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.AMAZONPAY_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Stripe AmazonPay wallet tests — connector not in AMAZONPAY_WALLET list"
            );
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
      ]["PaymentIntent"]("AmazonPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm AmazonPay redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AmazonPay"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = "amazon_pay";
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AmazonPay"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe AmazonPay Mandate Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.AMAZONPAY_WALLET
            )
          ) {
            skip = true;
            return;
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
      ]["PaymentIntent"]("AmazonPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm AmazonPay mandate redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AmazonPayMandate"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = "amazon_pay";
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AmazonPayMandate"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe AmazonPay - Invalid Currency error test", () => {
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
              CONNECTOR_LISTS.INCLUDE.AMAZONPAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    it("confirm-amazonpay-with-invalid-currency-should-fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("AmazonPay", "INR");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm AmazonPay with invalid currency", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["AmazonPayInvalidCurrency"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});

describe("Stripe Cashapp Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Stripe Cashapp Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.CASHAPP_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Stripe Cashapp wallet tests — connector not in CASHAPP_WALLET list"
            );
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
      ]["PaymentIntent"]("Cashapp");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Cashapp redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Cashapp"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const nextActionUrl = globalState.get("nextActionUrl");
      if (!nextActionUrl) {
        cy.log("Cashapp QR code flow — no redirect URL, skipping redirection");
        return;
      }
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleWalletRedirection(globalState, "cashapp", expected_redirection);
    });

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Cashapp"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe Cashapp Mandate Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.CASHAPP_WALLET
            )
          ) {
            skip = true;
            return;
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
      ]["PaymentIntent"]("Cashapp");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Cashapp mandate redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["CashappMandate"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const nextActionUrl = globalState.get("nextActionUrl");
      if (!nextActionUrl) {
        cy.log("Cashapp QR code flow — no redirect URL, skipping redirection");
        return;
      }
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleWalletRedirection(globalState, "cashapp", expected_redirection);
    });

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["CashappMandate"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe Cashapp - Invalid Currency error test", () => {
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
              CONNECTOR_LISTS.INCLUDE.CASHAPP_WALLET
            )
          ) {
            skip = true;
            return;
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

    it("confirm-cashapp-with-invalid-currency-should-fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("Cashapp", "INR");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Cashapp with invalid currency", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["CashappInvalidCurrency"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});

describe("Stripe RevolutPay Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Stripe RevolutPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.REVOLUTPAY_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Stripe RevolutPay wallet tests — connector not in REVOLUTPAY_WALLET list"
            );
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
      ]["PaymentIntent"]("RevolutPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm RevolutPay redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RevolutPay"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = "revolut_pay";
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RevolutPay"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe RevolutPay Mandate Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.REVOLUTPAY_WALLET
            )
          ) {
            skip = true;
            return;
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
      ]["PaymentIntent"]("RevolutPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm RevolutPay mandate redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RevolutPayMandate"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = "revolut_pay";
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

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RevolutPayMandate"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe RevolutPay - Invalid Currency error test", () => {
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
              CONNECTOR_LISTS.INCLUDE.REVOLUTPAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    it("confirm-revolutpay-with-invalid-currency-should-fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("RevolutPay", "INR");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm RevolutPay with invalid currency", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RevolutPayInvalidCurrency"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});

describe("Stripe WeChatPay Wallet tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Stripe WeChatPay Create and Confirm flow test", () => {
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
              CONNECTOR_LISTS.INCLUDE.WECHATPAY_WALLET
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            cy.log(
              "Skipping Stripe WeChatPay wallet tests — connector not in WECHATPAY_WALLET list"
            );
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
      ]["PaymentIntent"]("WeChatPay");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm WeChatPay redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WeChatPay"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("Handle wallet redirection", () => {
      const nextActionUrl = globalState.get("nextActionUrl");
      if (!nextActionUrl) {
        cy.log(
          "WeChatPay QR code flow — no redirect URL, skipping redirection"
        );
        return;
      }
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleWalletRedirection(
        globalState,
        "we_chat_pay",
        expected_redirection
      );
    });

    it("Sync payment status", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WeChatPay"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Stripe WeChatPay - Invalid Currency error test", () => {
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
              CONNECTOR_LISTS.INCLUDE.WECHATPAY_WALLET
            )
          ) {
            skip = true;
            return;
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

    it("confirm-wechatpay-with-invalid-currency-should-fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"]("WeChatPay", "INR");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm WeChatPay with invalid currency", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WeChatPayInvalidCurrency"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});
