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
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund flow test for No-3DS", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialRefund"];

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Fully Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment + Retrieve Payment after Confirmation + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      cy.step("Create and Confirm Payment", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Partially Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment + Retrieve Payment after Confirmation + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      cy.step("Create and Confirm Payment", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialRefund"];

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      const newData = {
        ...syncRefundData,
        Response: syncRefundData.ResponseCustom || syncRefundData.Response,
      };

      cy.step("Sync Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newData, globalState)
      );
    });
  });

  context("Card - Full Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentRefund"];

      const newRefundData = {
        ...refundData,
        Response: refundData.ResponseCustom || refundData.Response,
      };

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment + List Refunds", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      const newPartialRefundData = {
        ...partialRefundData,
        Response: partialRefundData.ResponseCustom || partialRefundData.Response,
      };

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );

      cy.step("List Refunds", () =>
        cy.listRefundCallTest(fixtures.listRefundCall, globalState)
      );
    });
  });

  context("Card - Full Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      const newPartialRefundData = {
        ...partialRefundData,
        Response: partialRefundData.ResponseCustom || partialRefundData.Response,
      };

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      const newPartialRefundData = {
        ...partialRefundData,
        Response: partialRefundData.ResponseCustom || partialRefundData.Response,
      };

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test", () => {
    it("CIT for Mandates Call + MIT for Mandates Call + MIT for Mandates Call - 2nd Attempt + Refund Payment + Sync Refund Payment", () => {
      const citData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateMultiUseNo3DSAutoCapture"];

      cy.step("CIT for Mandates Call", () =>
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          citData,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        )
      );

      if (!utils.should_continue_further(citData)) return;

      const mitData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.step("MIT for Mandates Call", () =>
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          mitData,
          6000,
          true,
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(mitData)) return;

      cy.step("MIT for Mandates Call - 2nd Attempt", () =>
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          mitData,
          6000,
          true,
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(mitData)) return;

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });
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
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund flow test for 3DS", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialRefund"];

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;
      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment + Handle Redirection + Retrieve Payment after Confirmation + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSAutoCapture"
      ];

      cy.step("Create and Confirm Payment", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Partially Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment + Handle Redirection + Retrieve Payment after Confirmation + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSAutoCapture"
      ];

      cy.step("Create and Confirm Payment", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];

      cy.step("Handle Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialRefund"];

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Full Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["manualPaymentRefund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture + Partial Refund Payment + Partial Refund Payment - 2nd Attempt + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      cy.step("Partial Refund Payment - 2nd Attempt", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Full Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture + Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });

  context("Card - Partial Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Handle Redirection + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture + Partial Refund Payment + Sync Refund Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );

      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["manualPaymentPartialRefund"];

      const newPartialRefundData = {
        ...partialRefundData,
        Request: { amount: partialRefundData.Request.amount / 2 },
      };

      cy.step("Partial Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState)
      );

      if (!utils.should_continue_further(partialRefundData)) return;

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.step("Sync Refund Payment", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });
});
