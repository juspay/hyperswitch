import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

// Every Peach Payments APM brand, modelled as its own payment_method_type.
// Brands whose config carries TRIGGER_SKIP (region-specific entities, real
// voucher PINs, account credentials) skip automatically until enabled.
const APM_FLOWS = [
  // South African bank transfers (ZAR)
  {
    name: "Capitec Pay",
    pmGroup: "bank_transfer_pm",
    configKey: "CapitecPay",
    confirmType: "bank_transfer",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "PayShap",
    pmGroup: "bank_transfer_pm",
    configKey: "PayShap",
    confirmType: "bank_transfer",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "Nedbank Direct EFT",
    pmGroup: "bank_transfer_pm",
    configKey: "NedbankDirectEft",
    confirmType: "bank_transfer",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "Peach EFT",
    pmGroup: "bank_transfer_pm",
    configKey: "PeachEft",
    confirmType: "bank_transfer",
    expectedStatus: "requires_customer_action",
  },
  // Buy now pay later / store credit (ZAR)
  {
    name: "Payflex",
    pmGroup: "pay_later_pm",
    configKey: "Payflex",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "ZeroPay",
    pmGroup: "pay_later_pm",
    configKey: "ZeroPay",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "Float",
    pmGroup: "pay_later_pm",
    configKey: "Float",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "HappyPay",
    pmGroup: "pay_later_pm",
    configKey: "HappyPay",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "Mobicred",
    pmGroup: "pay_later_pm",
    configKey: "Mobicred",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "RCS",
    pmGroup: "pay_later_pm",
    configKey: "Rcs",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "A+ Store Cards",
    pmGroup: "pay_later_pm",
    configKey: "APlus",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  // Wallets / QR (ZAR, KES, MUR)
  {
    name: "Scan to Pay",
    pmGroup: "wallet_pm",
    configKey: "ScanToPay",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "M-PESA",
    pmGroup: "wallet_pm",
    configKey: "Mpesa",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "blink by Emtel",
    pmGroup: "wallet_pm",
    configKey: "BlinkByEmtel",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "MCB Juice",
    pmGroup: "wallet_pm",
    configKey: "McbJuice",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  {
    name: "MauCAS",
    pmGroup: "wallet_pm",
    configKey: "Maucas",
    confirmType: "redirect",
    expectedStatus: "requires_customer_action",
  },
  // Voucher (ZAR) - synchronous flow, no redirect
  {
    name: "1ForYou",
    pmGroup: "voucher_pm",
    configKey: "OneForYou",
    confirmType: "voucher",
    expectedStatus: "succeeded",
  },
  // Crypto (ZAR)
  {
    name: "MoneyBadger",
    pmGroup: "crypto_pm",
    configKey: "MoneyBadger",
    confirmType: "crypto",
    expectedStatus: "requires_customer_action",
  },
];

function getPaymentIntentData(pmGroup, configKey) {
  const group = getConnectorDetails(globalState.get("connectorId"))[pmGroup];
  return typeof group["PaymentIntent"] === "function"
    ? group["PaymentIntent"](configKey)
    : group["PaymentIntent"];
}

function confirmApm(confirmType, data) {
  switch (confirmType) {
    case "bank_transfer":
      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      break;
    case "voucher":
      cy.confirmVoucherCallTest(fixtures.confirmBody, data, true, globalState);
      break;
    case "crypto":
      cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);
      break;
    case "redirect":
    default:
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      break;
  }
}

describe("Peach Payments Alternative Payment Methods", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");
        if (connector !== "peachpayments") {
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

  APM_FLOWS.forEach((flow) => {
    context(`${flow.name} payment flow`, () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-call-test", () => {
        const data = getPaymentIntentData(flow.pmGroup, flow.configKey);

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it(`confirm-${flow.configKey}-call-test`, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          flow.pmGroup
        ][flow.configKey];

        confirmApm(flow.confirmType, data);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          flow.pmGroup
        ][flow.configKey];

        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: flow.expectedStatus,
        });
      });
    });
  });

  context("PayShap manual capture rejection", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getPaymentIntentData("bank_transfer_pm", "PayShap");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-manual-capture-rejected-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayShapManualCapture"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
    });
  });

  // Server-to-server one step flow: create the payment with confirm: true
  // (no SDK / client_secret involved), as opposed to the confirm: false +
  // SDK-confirm flows exercised by the per-brand contexts above
  context("PayShap server-to-server create and confirm", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayShap"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayShap"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });
});
