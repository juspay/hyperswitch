import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - MultiUse Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {
    const citData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MandateMultiUseNo3DSAutoCapture"
      ];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const mitData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MITAutoCapture"
      ];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {
    const citData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MandateMultiUseNo3DSManualCapture"
      ];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "manual",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const captureData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    const mitData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MITManualCapture"
      ];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "manual",
      globalState
    );

    if (!utils.should_continue_further(mitData)) return;

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "manual",
      globalState
    );

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);
  });

  it("Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {
    const citData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MandateMultiUseNo3DSManualCapture"
      ];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "manual",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const captureData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    const mitData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MITAutoCapture"
      ];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );
  });
});