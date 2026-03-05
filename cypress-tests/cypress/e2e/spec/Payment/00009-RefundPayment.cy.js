import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Refund Payment", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund flow test for No-3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Fully Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
        let shouldContinue = true;

        step("Create and Confirm Payment", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        step("Refund Payment", shouldContinue, () => {
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        step("Sync Refund Payment", shouldContinue, () => {
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context(
    "Partially Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
        let shouldContinue = true;

        step("Create and Confirm Payment", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        step("Partial Refund Payment", shouldContinue, () => {
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        step("Sync Refund Payment", shouldContinue, () => {
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          const newData = {
            ...syncRefundData,
            Response: syncRefundData.ResponseCustom || syncRefundData.Response,
          };
          cy.refundCallTest(fixtures.refundBody, newData, globalState);
        });
      });
    }
  );

  context("Card - Full Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Capture", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
      });

      step("Refund Payment", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["manualPaymentRefund"];
        const newRefundData = {
          ...refundData,
          Response: refundData.ResponseCustom || refundData.Response,
        };
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment -> List Refunds", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Capture", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
      });

      step("Partial Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });

      step("List Refunds", shouldContinue, () => {
        cy.listRefundCallTest(fixtures.listRefundCall, globalState);
      });
    });
  });

  context("Card - Full Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Capture Payment", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.captureCallTest(
          fixtures.captureBody,
          partialCaptureData,
          globalState
        );
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Partial Capture", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
      });

      step("Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Capture Payment", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.captureCallTest(
          fixtures.captureBody,
          partialCaptureData,
          globalState
        );
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Partial Capture", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
      });

      step("Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("CIT for Mandates Call -> MIT for Mandates Call -> MIT for Mandates Call - 2nd Attempt -> Refund Payment -> Sync Refund Payment", () => {
        let shouldContinue = true;

        step("CIT for Mandates Call", shouldContinue, () => {
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
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("MIT for Mandates Call", shouldContinue, () => {
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
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("MIT for Mandates Call - 2nd Attempt", shouldContinue, () => {
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
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Refund Payment", shouldContinue, () => {
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        step("Sync Refund Payment", shouldContinue, () => {
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
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
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Refund Payment", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund flow test for 3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      step("Refund Payment", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Partially Refund Card-ThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
        let shouldContinue = true;

        step("Create and Confirm Payment", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Handle Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        step("Partial Refund Payment", shouldContinue, () => {
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        step("Sync Refund Payment", shouldContinue, () => {
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context("Card - Full Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Capture", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
      });

      step("Refund Payment", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["manualPaymentRefund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Capture", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
      });

      step("Partial Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Partial Refund Payment - 2nd Attempt", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Full Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Capture Payment", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.captureCallTest(
          fixtures.captureBody,
          partialCaptureData,
          globalState
        );
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Partial Capture", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
      });

      step("Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Partial Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      step("Partial Capture Payment", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.captureCallTest(
          fixtures.captureBody,
          partialCaptureData,
          globalState
        );
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment after Partial Capture", shouldContinue, () => {
        const partialCaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialCapture"];
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
      });

      step("Partial Refund Payment", shouldContinue, () => {
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Request: { amount: partialRefundData.Request.amount / 2 },
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund Payment", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
