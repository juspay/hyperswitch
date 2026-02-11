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

  it("should complete NoThreeDS automatic CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    // Confirm No 3DS MIT (first)
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Confirm No 3DS MIT (second)
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete NoThreeDS manual CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Confirm No 3DS MIT 1
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITManualCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture MIT 1
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Confirm No 3DS MIT 2
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture MIT 2
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);
  });

  it("should complete ThreeDS manual CIT with automatic MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);
  });
});