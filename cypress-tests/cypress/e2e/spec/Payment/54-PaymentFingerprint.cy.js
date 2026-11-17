import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

const confirmDataFor = (flowKey) =>
  getConnectorDetails(globalState.get("connectorId"))["card_pm"][flowKey];

const runFingerprintFlow = (flowKey, retrieveAfterFailedConfirm = false) => {
  let shouldContinue = true;

  cy.step("Create Payment Intent", () => {
    const data = confirmDataFor("PaymentIntent");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!should_continue_further(data)) {
      shouldContinue = false;
    }
  });

  cy.step("Confirm Payment", () => {
    if (!shouldContinue) {
      cy.task("cli_log", "Skipping step: Confirm Payment");
      return;
    }
    const confirmData = confirmDataFor(flowKey);

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!retrieveAfterFailedConfirm && !should_continue_further(confirmData)) {
      shouldContinue = false;
    }
  });

  cy.step("Retrieve Payment", () => {
    if (!shouldContinue) {
      cy.task("cli_log", "Skipping step: Retrieve Payment");
      return;
    }

    cy.retrievePaymentCallTest({
      globalState,
      data: confirmDataFor(flowKey),
      expectFingerprint: true,
    });
  });
};

describe("Card - Payment fingerprint generation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Succeeded card payment gets a fingerprint", () => {
    before(function () {
      if (
        confirmDataFor("No3DSAutoCapture")?.Response?.body?.status !==
        "succeeded"
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      runFingerprintFlow("No3DSAutoCapture");
    });
  });

  context("Failed card payment still records the fingerprint type", () => {
    before(function () {
      if (
        confirmDataFor("No3DSFailPayment")?.Response?.body?.status !== "failed"
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      runFingerprintFlow("No3DSFailPayment", true);
    });
  });
});
