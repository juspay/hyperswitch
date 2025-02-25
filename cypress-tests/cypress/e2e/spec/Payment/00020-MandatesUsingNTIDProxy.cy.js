import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Mandates using Network Transaction Id flow test", () => {
  before(function () {
    // Changed to regular function instead of arrow function
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip the test if the connector is not in the inclusion list
        // This is done because only cybersource is known to support at present
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
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

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
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

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
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

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
    "Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test",
    () => {
      let shouldContinue = true;

      it("Confirm No 3DS MIT 1", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT 2", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

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
    "Card - ThreeDS Create and Confirm Manual multiple MITs payment flow",
    () => {
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

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
