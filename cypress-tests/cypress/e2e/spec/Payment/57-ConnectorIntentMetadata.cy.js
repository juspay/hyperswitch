import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Connector Intent Metadata payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.CONNECTOR_INTENT_METADATA
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - NoThreeDS CIT and MIT with connector metadata", function () {
    before(
      "skip connectors without error on requires action metadata support",
      function () {
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.ERROR_ON_REQUIRES_ACTION
          )
        ) {
          this.skip();
        }
      }
    );

    it("Confirm No 3DS CIT -> Confirm MIT with connector metadata -> retrieve-payment-call-test", () => {
      let shouldContinue = true;

      cy.step("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm MIT with connector metadata", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm MIT with connector metadata"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCaptureWithErrorOnRequiresAction"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          true,
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("retrieve-payment-call-test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCaptureWithErrorOnRequiresAction"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Card - Create+Confirm payment with connector metadata", function () {
    before(
      "skip connectors without connector metadata echo support",
      function () {
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.PEACHPAYMENTS_CONNECTOR_METADATA
          )
        ) {
          this.skip();
        }
      }
    );

    it("Create+Confirm Payment with connector metadata -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create+Confirm Payment with connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadata"];

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

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadata"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Card - Account funded transaction with purpose of payment metadata",
    function () {
      before(
        "skip connectors without AFT purpose of payment support",
        function () {
          if (
            utils.shouldIncludeConnector(
              globalState.get("connectorId"),
              utils.CONNECTOR_LISTS.INCLUDE.AFT_PURPOSE_OF_PAYMENT
            )
          ) {
            this.skip();
          }
        }
      );

      it("Create+Confirm AFT payment with connector metadata (sandbox declines AFT: failed status expected, connector metadata echo asserted)", () => {
        // The connector's sandbox is not AFT-enabled: Hyperswitch validates and echoes
        // connector_metadata, but the sandbox declines the transfer, so the payment ends
        // in "failed" with the documented sandbox error code (expected values live in the
        // connector config). This test asserts the connector_metadata echo, not payment
        // success.
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadata"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment without connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataMissing"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment with unknown field in connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataUnknownField"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - Account funded transaction with funding transaction type and payment purpose metadata",
    function () {
      before(
        "skip connectors without AFT funding transaction type support",
        function () {
          if (
            utils.shouldIncludeConnector(
              globalState.get("connectorId"),
              utils.CONNECTOR_LISTS.INCLUDE.AFT_FUNDING_TRANSACTION_TYPE
            )
          ) {
            this.skip();
          }
        }
      );

      it("Create+Confirm AFT payment with connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadata"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment without connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataMissing"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment with missing funding transaction type", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataMissingFundingTransactionType"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment with missing payment purpose", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataMissingPaymentPurpose"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Create+Confirm AFT payment with unknown field in connector metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorIntentMetadataUnknownField"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    }
  );
});
