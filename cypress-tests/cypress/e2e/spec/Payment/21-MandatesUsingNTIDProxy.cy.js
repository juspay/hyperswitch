import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Mandates using Network Transaction Id flow test", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");
        // Skip the test if the connector is not in the inclusion list
        // Connectors that support Mandates Using NTID Proxy
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.MANDATES_USING_NTID_PROXY
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create and Confirm Automatic MIT payment flow test",
    () => {
      it("Confirm No 3DS MIT", function () {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        if (!utils.should_continue_further(data)) {
          this.skip();
        }

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual MIT payment flow test",
    () => {
      it("Confirm No 3DS MIT", function () {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        if (!utils.should_continue_further(data)) {
          this.skip();
        }

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      it("Confirm No 3DS MIT -> Confirm No 3DS MIT", function () {
        let shouldContinue = true;

        cy.step("Confirm No 3DS MIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "automatic",
            globalState
          );
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          if (!utils.should_continue_further(data)) {
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "automatic",
            globalState
          );
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test",
    () => {
      it("Confirm No 3DS MIT 1 -> mit-capture-call-test -> Confirm No 3DS MIT 2 -> mit-capture-call-test", function () {
        let shouldContinue = true;

        cy.step("Confirm No 3DS MIT 1", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "manual",
            globalState
          );
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            return;
          }

          cy.captureCallTest(fixtures.captureBody, data, globalState);
        });

        cy.step("Confirm No 3DS MIT 2", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT 2");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "manual",
            globalState
          );
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          if (!utils.should_continue_further(data)) {
            return;
          }

          cy.captureCallTest(fixtures.captureBody, data, globalState);
        });
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      it("Confirm No 3DS MIT -> Confirm No 3DS MIT", function () {
        let shouldContinue = true;

        cy.step("Confirm No 3DS MIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "automatic",
            globalState
          );
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          if (!utils.should_continue_further(data)) {
            return;
          }

          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            data,
            6000,
            true,
            "automatic",
            globalState
          );
        });
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Manual multiple MITs payment flow",
    () => {
      it("Confirm No 3DS MIT", function () {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        if (!utils.should_continue_further(data)) {
          this.skip();
        }

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );
});
