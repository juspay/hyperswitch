import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Connector Intent Metadata payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
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
    "Card - Payment with purpose of payment connector metadata",
    function () {
      before(
        "skip connectors without purpose of payment metadata support",
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

      it("Create+Confirm payment with connector metadata", () => {
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

      it("Create+Confirm payment with unknown field in connector metadata", () => {
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
    "Card - Payment with funding transaction type and payment purpose connector metadata",
    function () {
      before(
        "skip connectors without funding transaction type metadata support",
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

      it("Create+Confirm payment with connector metadata", () => {
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

      it("Create+Confirm payment without connector metadata", () => {
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

      it("Create+Confirm payment with connector metadata missing funding transaction type", () => {
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

      it("Create+Confirm payment with connector metadata missing payment purpose", () => {
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

      it("Create+Confirm payment with unknown field in connector metadata", () => {
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
