import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
      it("MIT - Auto Capture using NTID", () => {
        let shouldContinue = true;

        step("MIT - Auto Capture using NTID", shouldContinue, () => {
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
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual MIT payment flow test",
    () => {
      it("MIT - Manual Capture using NTID", () => {
        let shouldContinue = true;

        step("MIT - Manual Capture using NTID", shouldContinue, () => {
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
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      it("MIT 1 - Auto Capture using NTID -> MIT 2 - Auto Capture using NTID", () => {
        let shouldContinue = true;

        step("MIT 1 - Auto Capture using NTID", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Auto Capture using NTID", shouldContinue, () => {
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
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test",
    () => {
      it("MIT 1 - Manual Capture using NTID -> Capture MIT 1 Payment -> MIT 2 - Manual Capture using NTID -> Capture MIT 2 Payment", () => {
        let shouldContinue = true;

        step("MIT 1 - Manual Capture using NTID", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];
          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            mitData,
            6000,
            true,
            "manual",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Capture MIT 1 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Manual Capture using NTID", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];
          cy.mitUsingNTID(
            fixtures.ntidConfirmBody,
            mitData,
            6000,
            true,
            "manual",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Capture MIT 2 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        });
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      it("MIT 1 - Auto Capture using NTID -> MIT 2 - Auto Capture using NTID", () => {
        let shouldContinue = true;

        step("MIT 1 - Auto Capture using NTID", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Auto Capture using NTID", shouldContinue, () => {
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
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Manual multiple MITs payment flow",
    () => {
      it("MIT - Auto Capture using NTID", () => {
        let shouldContinue = true;

        step("MIT - Auto Capture using NTID", shouldContinue, () => {
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
      });
    }
  );
});
