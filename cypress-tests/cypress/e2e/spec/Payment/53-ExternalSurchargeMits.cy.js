import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  captureFullAmount,
  cardData,
  refundShareOfOrderAmount,
  shouldSkipExternalSurcharge,
} from "../../../utils/externalSurcharge";
import { should_continue_further } from "../../configs/Payment/Utils";

let globalState;
let originalCustomerId;

// Merchant-initiated payments (off_session) on a saved card. There is no
// eligibility step: the surcharge is calculated at confirm, from the card's
// IIN and the billing address. If it cannot be calculated, the payment goes
// ahead with no surcharge and no error, so every MIT here must carry one.
//
// Needs the surcharge connector that 03-ConnectorCreate creates from the
// "interpayments" entry in creds.json; the spec skips itself without it.
describe("External surcharge - merchant-initiated payments", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        originalCustomerId = globalState.get("customerId");
        return shouldSkipExternalSurcharge(globalState);
      })
      .then((shouldSkip) => {
        skip = shouldSkip;
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("unlink the surcharge connector and restore the customer", () => {
    cy.unlinkSurchargeConnectorFromProfile(globalState);
    globalState.set("customerId", originalCustomerId);
    cy.task("setGlobalState", globalState.data);
  });

  it("Point the profile at the surcharge connector", () => {
    cy.linkSurchargeConnectorToProfile(globalState);
  });

  context("Save the card with a customer-initiated payment", () => {
    it("Create Customer -> Create Payment Intent -> Confirm CIT -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Intent (off_session)", () => {
        const data = cardData(globalState, "PaymentIntentOffSession");
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

      cy.step("Confirm CIT (saves the card for off-session use)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm CIT");
          return;
        }
        const data = cardData(
          globalState,
          "PaymentMethodIdMandateNo3DSAutoCapture"
        );
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve the CIT payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve the CIT payment");
          return;
        }
        cy.retrievePaymentCallTest({
          globalState,
          data: cardData(globalState, "PaymentMethodIdMandateNo3DSAutoCapture"),
        });
      });
    });
  });

  context("Automatic capture MIT", () => {
    it("MIT -> Retrieve Payment -> Partial refund", () => {
      // Compare with this payment's own surcharge, not an earlier one.
      cy.resetSurchargeState(globalState);

      cy.step("Confirm MIT (the response carries the surcharge)", () => {
        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          cardData(globalState, "ExternalSurchargeMITAutoCapture"),
          true,
          "automatic",
          globalState
        );
      });

      cy.step("Retrieve Payment (the surcharge is persisted)", () => {
        cy.retrievePaymentCallTest({
          globalState,
          data: cardData(globalState, "ExternalSurchargeRetrieve"),
        });
      });

      cy.step("Partial refund carries a share of the surcharge", () => {
        refundShareOfOrderAmount(globalState, 0.4, "mitSurchargeRefund");
      });
    });
  });

  context("Manual capture MIT", () => {
    it("MIT -> Capture -> Retrieve Payment -> Partial refund", () => {
      cy.resetSurchargeState(globalState);

      cy.step("Confirm MIT (the response carries the surcharge)", () => {
        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          cardData(globalState, "ExternalSurchargeMITManualCapture"),
          true,
          "manual",
          globalState
        );
      });

      cy.step("Capture the full charged amount", () => {
        captureFullAmount(globalState);
      });

      cy.step("Retrieve Payment (the surcharge is persisted)", () => {
        cy.retrievePaymentCallTest({
          globalState,
          data: cardData(globalState, "ExternalSurchargeRetrieve"),
        });
      });

      cy.step("Partial refund carries a share of the surcharge", () => {
        refundShareOfOrderAmount(globalState, 0.4, "mitManualSurchargeRefund");
      });
    });
  });
});
