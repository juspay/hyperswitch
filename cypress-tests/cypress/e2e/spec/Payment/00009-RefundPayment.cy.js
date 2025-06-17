import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Refund flow - No 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Full Refund flow test for No-3DS", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - Partial Refund flow test for No-3DS", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context(
    "Fully Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("sync-refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];

        cy.syncRefundCallTest(data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Partially Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("sync-refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];

        cy.syncRefundCallTest(data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context("Card - Full Refund for fully captured No-3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - Partial Refund for fully captured No-3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("list-refund-call-test", () => {
      cy.listRefundCallTest(fixtures.listRefundCall, globalState);
    });
  });

  context("Card - Full Refund for partially captured No-3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - partial Refund for partially captured No-3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context(
    "Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateMultiUseNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("sync-refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];

        cy.syncRefundCallTest(data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );
});

describe("Card - Refund flow - 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Full Refund flow test for 3DS", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

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

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - Partial Refund flow test for 3DS", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

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

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create+confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context(
    "Partially Refund Card-ThreeDS payment flow test Create+Confirm",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle redirection", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        cy.retrievePaymentCallTest(globalState, data);
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("sync-refund-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];

        cy.syncRefundCallTest(data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context("Card - Full Refund for fully captured 3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - Partial Refund for fully captured 3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - Full Refund for partially captured 3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Card - partial Refund for partially captured 3DS payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentPartialRefund"];

      const newData = {
        ...data,
        Request: { amount: data.Request.amount / 2 },
      };

      cy.refundCallTest(fixtures.refundBody, newData, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
