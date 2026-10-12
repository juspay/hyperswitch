import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Processing Method ID payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.PROCESSING_METHOD_ID
          )
        ) {
          skip = true;
          return;
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

  context("Card-NoThreeDS payment with metadata.processing_method_id", () => {
    it("Create Payment Intent -> Payment Methods -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent with processing_method_id metadata",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentWithProcessingMethodId"];

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
        }
      );

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with processing_method_id metadata", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithProcessingMethodId"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with processing_method_id metadata", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithProcessingMethodId"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Card - Mandate CIT payment with metadata.processing_method_id",
    () => {
      let shouldContinue = true;

      it("Confirm No 3DS CIT with processing_method_id metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSAutoCaptureWithProcessingMethodId"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment with processing_method_id metadata", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSAutoCaptureWithProcessingMethodId"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );
});
