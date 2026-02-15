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

  it("Card - Full Refund flow test for No-3DS", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund flow test for No-3DS", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Fully Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.retrievePaymentCallTest({ globalState, data });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Partially Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.retrievePaymentCallTest({ globalState, data });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    const newData = {
      ...syncRefundData,
      Response: syncRefundData.ResponseCustom || syncRefundData.Response,
    };

    cy.refundCallTest(fixtures.refundBody, newData, globalState);
  });

  it("Card - Full Refund for fully captured No-3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["manualPaymentRefund"];

    const newRefundData = {
      ...refundData,
      Response: refundData.ResponseCustom || refundData.Response,
    };

    cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund for fully captured No-3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    const newPartialRefundData = {
      ...partialRefundData,
      Response: partialRefundData.ResponseCustom || partialRefundData.Response,
    };

    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);

    cy.listRefundCallTest(fixtures.listRefundCall, globalState);
  });

  it("Card - Full Refund for partially captured No-3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    const newPartialRefundData = {
      ...partialRefundData,
      Response: partialRefundData.ResponseCustom || partialRefundData.Response,
    };

    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund for partially captured No-3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    const newPartialRefundData = {
      ...partialRefundData,
      Response: partialRefundData.ResponseCustom || partialRefundData.Response,
    };

    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MandateMultiUseNo3DSAutoCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(mitData)) return;

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(mitData)) return;

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
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

  it("Card - Full Refund flow test for 3DS", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSAutoCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund flow test for 3DS", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSAutoCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "3DSAutoCapture"
    ];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Partially Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "3DSAutoCapture"
    ];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Full Refund for fully captured 3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["manualPaymentRefund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund for fully captured 3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Full Refund for partially captured 3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("Card - Partial Refund for partially captured 3DS payment", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSManualCapture"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    const partialRefundData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["manualPaymentPartialRefund"];

    const newPartialRefundData = {
      ...partialRefundData,
      Request: { amount: partialRefundData.Request.amount / 2 },
    };

    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    if (!utils.should_continue_further(partialRefundData)) return;

    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SyncRefund"];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });
});
