import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

describe("Wallet Mandate tests", () => {
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
            CONNECTOR_LISTS.INCLUDE.ADYEN_WALLET_MANDATE
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-paypal-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-paypal-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaypalWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "EUR",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "KRW",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-kakaopay-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-kakaopay-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["KakaoPayWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "KRW",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "PHP",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-gcash-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-gcash-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GcashWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "PHP",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "VND",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-momo-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["MomoWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-momo-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["MomoWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "VND",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "CHF",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-twint-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-twint-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["TwintWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "CHF",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "NOK",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-vipps-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["VippsWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-vipps-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["VippsWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "NOK",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "IDR",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-dana-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-dana-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DanaWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "IDR",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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

    it("create-wallet-mandate-payment-intent-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "IDR",
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        modifiedData,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-gopay-wallet-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMandateCIT"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
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
      ]["WalletMandateSingleUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
      });

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("confirm-gopay-wallet-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["GoPayWalletMITAutoCapture"];

      const modifiedData = {
        ...data,
        Request: {
          ...data.Request,
          currency: "IDR",
        },
      };

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        modifiedData,
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
