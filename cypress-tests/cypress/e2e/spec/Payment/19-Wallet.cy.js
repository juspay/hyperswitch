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
    const shouldContinue = true;

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
      let shouldContinue = true;

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
          "wallet_pm"
        ]["Mifinity"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});

describe("PayPal Wallet tests", () => {
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

  context("PayPal Create and Confirm flow test", () => {
    let shouldContinue = true;

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

    it.skip("Handle wallet redirection", () => {
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

    it.skip("Handle wallet redirection", () => {
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
      ]["PaypalRedirectMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });
});
