import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

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
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - Partial Refund flow test for No-3DS", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        1200,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        1200,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context(
    "Fully Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("sync-refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.syncRefundCallTest(req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Partially Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          3000,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          3000,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("sync-refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.syncRefundCallTest(req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context("Card - Full Refund for fully captured No-3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - Partial Refund for fully captured No-3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        3000,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        3000,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
    it("list-refund-call-test", () => {
      cy.listRefundCallTest(fixtures.listRefundCall, globalState);
    });
  });

  context("Card - Full Refund for partially captured No-3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - partial Refund for partially captured No-3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context(
    "Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateMultiUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + req_data.card);
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          7000,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("sync-refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.syncRefundCallTest(req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
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
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - Partial Refund flow test for 3DS", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        1200,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        1200,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create+confirm-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context(
    "Partially Refund Card-ThreeDS payment flow test Create+Confirm",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          3000,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.refundCallTest(
          fixtures.refundBody,
          req_data,
          res_data,
          3000,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("sync-refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.syncRefundCallTest(req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context("Card - Full Refund for fully captured 3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - Partial Refund for fully captured 3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        5000,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        1500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - Full Refund for partially captured 3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("Card - partial Refund for partially captured 3DS payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "manual",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PartialCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        100,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        50,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("sync-refund-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.syncRefundCallTest(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });
});
