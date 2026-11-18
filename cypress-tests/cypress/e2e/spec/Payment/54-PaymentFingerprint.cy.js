import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

const confirmDataFor = (flowKey) =>
  getConnectorDetails(globalState.get("connectorId"))["card_pm"][flowKey];

const runFingerprintFlow = (flowKey, isFailedPaymentFlow = false) => {
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

    shouldContinue = isFailedPaymentFlow
      ? confirmData.Response?.status === 200
      : should_continue_further(confirmData);
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
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        return cy.vaultAvailabilityCheck(globalState);
      })
      .then((vaultAvailable) => {
        if (!vaultAvailable) {
          cy.task(
            "cli_log",
            "Vault is not available in this environment, skipping fingerprint spec"
          );
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

  context("Succeeded card payment gets a fingerprint", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      runFingerprintFlow("No3DSAutoCapture");
    });
  });

  context("Failed card payment still records the fingerprint type", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      runFingerprintFlow("No3DSFailPayment", true);
    });
  });
});
