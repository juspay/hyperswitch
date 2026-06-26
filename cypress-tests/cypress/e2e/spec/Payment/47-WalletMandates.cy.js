import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

describe("Wallet Mandate tests", () => {
  before("seed global state", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("PayPal Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support PayPal wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAYPAL_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("paypal-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("KakaoPay Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support KakaoPay wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.KAKAO_PAY_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("kakaopay-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("Gcash Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support Gcash wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.GCASH_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("gcash-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("Twint Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support Twint wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.TWINT_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("twint-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("Dana Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support Dana wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.DANA_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("dana-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("GoPay Wallet Mandate CIT flow test", () => {
    let shouldContinue = true;

    before(
      "skip if connector does not support GoPay wallet mandates",
      function () {
        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.GOPAY_WALLET_MANDATE
          )
        ) {
          this.skip();
        }
      }
    );

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("gopay-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMandateCIT"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});
