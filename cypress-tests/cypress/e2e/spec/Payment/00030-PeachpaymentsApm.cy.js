import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("PeachPayments Alternative Payment Methods", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  // ============================================================================
  // South African APMs (ZAR) - Working in sandbox
  // ============================================================================

  context("PeachPayments APM - PayShap (Nedbank)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm PayShap payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["LocalBankTransfer"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Peach EFT (Bank Redirect)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Peach EFT payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PeachEft"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Capitec Pay", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Capitec Pay payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["CapitecPay"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Payflex (BNPL)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Payflex payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Payflex"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - ZeroPay (BNPL)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm ZeroPay payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["ZeroPay"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Float (BNPL)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Float payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Float"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Happy Pay (BNPL)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Happy Pay payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["HappyPay"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Masterpass (QR Code)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Masterpass payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Masterpass"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - MoneyBadger (Crypto)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm MoneyBadger payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["MoneyBadger"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  // ============================================================================
  // South African APMs (ZAR) - May require specific sandbox setup
  // ============================================================================

  context("PeachPayments APM - 1Voucher (Voucher)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm 1Voucher payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["OneVoucher"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - Mobicred", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm Mobicred payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Mobicred"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - RCS Cards", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm RCS payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["RCS"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - A+ Store Cards", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm A+ payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["APlus"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  // ============================================================================
  // International APMs - May require region-specific entity
  // ============================================================================

  context("PeachPayments APM - M-PESA (Kenya)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm M-PESA payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Mpesa"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - blink by Emtel (Mauritius)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm blink by Emtel payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["BlinkByEmtel"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("PeachPayments APM - MCB Juice (Mauritius)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

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

    it("Confirm MCB Juice payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["McbJuice"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
