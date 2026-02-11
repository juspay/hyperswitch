import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SingleUse Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete NoThreeDS automatic CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete NoThreeDS manual CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITManualCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture MIT
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // List mandates
    cy.listMandateCallTest(globalState);
  });

  it("should complete NoThreeDS manual CIT with automatic MIT payment flow", () => {
    // Create No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // List mandates
    cy.listMandateCallTest(globalState);
  });
});