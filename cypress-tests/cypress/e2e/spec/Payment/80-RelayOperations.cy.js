import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  should_continue_further,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("Relay Operations", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connectorId,
            CONNECTOR_LISTS.INCLUDE.RELAY_OPERATIONS
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          cy.log(
            `Skipping relay operation tests — connector not in RELAY_OPERATIONS list`
          );
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Relay Refund — auto-capture payment", () => {
    it("Create Payment → Confirm → POST /relay (refund) → GET /relay → GET /relay?force_sync", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("Confirm Payment (no 3DS, auto-capture)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("POST /relay (refund)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: POST /relay refund");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "relay_pm"
        ]["RefundRelay"];

        cy.relayCallTest(fixtures.relayBody, data, globalState);

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("GET /relay/{relay_id}", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: GET /relay");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "relay_pm"
        ]["RetrieveRelay"];

        cy.retrieveRelayCallTest(data, globalState);
      });

      cy.step("GET /relay/{relay_id}?force_sync=true", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: GET /relay force_sync");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "relay_pm"
        ]["RetrieveRelay"];

        cy.retrieveRelayCallTest(data, globalState, true);
      });
    });
  });

  context("Relay Capture — manual capture payment", () => {
    it("Create Payment (manual) → Confirm → POST /relay (capture) → GET /relay", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent (manual capture)", () => {
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

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("Confirm Payment (no 3DS, manual capture)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("POST /relay (capture)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: POST /relay capture");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "relay_pm"
        ]["CaptureRelay"];

        cy.relayCallTest(fixtures.relayBody, data, globalState);

        if (!should_continue_further(data)) shouldContinue = false;
      });

      cy.step("GET /relay/{relay_id}", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: GET /relay");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "relay_pm"
        ]["RetrieveCaptureRelay"];

        cy.retrieveRelayCallTest(data, globalState);
      });
    });
  });

  context("Relay Error — invalid request validation", () => {
    it("POST /relay without connector_id → 400 IR_06", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "relay_pm"
      ]["MissingConnectorId"];

      cy.relayCallTest({}, data, globalState, true);
    });

    it("POST /relay with invalid relay type → 400 IR_06", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "relay_pm"
      ]["InvalidRelayType"];

      cy.relayCallTest({}, data, globalState, true);
    });

    it("GET /relay with non-existent relay_id → 400 IR_37", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "relay_pm"
      ]["RelayNotFound"];

      cy.retrieveRelayCallTest(
        data,
        globalState,
        false,
        "relay_InvalidIdForTesting123"
      );
    });
  });
});
