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
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        cy.log(
          `[WalletMandates] connectorId="${connector}"`
        );
        cy.log(
          `[WalletMandates] ADYEN_WALLET_MANDATE list=${JSON.stringify(
            CONNECTOR_LISTS.INCLUDE.ADYEN_WALLET_MANDATE
          )}`
        );

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.ADYEN_WALLET_MANDATE
          )
        ) {
          cy.log(
            `[WalletMandates] SKIP — connector "${connector}" is NOT in ADYEN_WALLET_MANDATE`
          );
          skip = true;
        } else {
          cy.log(
            `[WalletMandates] RUN — connector "${connector}" IS in ADYEN_WALLET_MANDATE`
          );
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

  context("PayPal Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("paypal-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("KakaoPay Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("kakaopay-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Gcash Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("gcash-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Momo Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("momo-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["MomoWalletMandateCIT"];

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
      ]["MomoWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("momo-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["MomoWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["MomoWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Twint Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("twint-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Vipps Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("vipps-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["VippsWalletMandateCIT"];

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
      ]["VippsWalletMandateCIT"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("vipps-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["VippsWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["VippsWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Dana Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("dana-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("GoPay Wallet Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

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

    it("gopay-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMITAutoCapture"];

      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-payment-after-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMITAutoCapture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
});
