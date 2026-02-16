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

  it("Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MandateSingleUseNo3DSAutoCapture"];

    cy.task("cli_log", "CIT for Mandate Call");
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

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.task("cli_log", "MIT for Mandate Call");
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
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MandateSingleUseNo3DSManualCapture"];

    cy.task("cli_log", "CIT for Mandate Call");
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

    const citCaptureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "CIT Capture Call");
    cy.captureCallTest(fixtures.captureBody, citCaptureData, globalState);

    if (!utils.should_continue_further(citCaptureData)) return;

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITManualCapture"];

    cy.task("cli_log", "MIT for Mandate Call");
    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "manual",
      globalState
    );

    if (!utils.should_continue_further(mitData)) return;

    const mitCaptureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "MIT Capture Call");
    cy.captureCallTest(fixtures.captureBody, mitCaptureData, globalState);

    if (!utils.should_continue_further(mitCaptureData)) return;

    cy.task("cli_log", "List Mandates Call");
    cy.listMandateCallTest(globalState);
  });

  it("Card - No threeDS Create + Confirm Manual CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MandateSingleUseNo3DSManualCapture"];

    cy.task("cli_log", "CIT for Mandate Call");
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

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "CIT Capture Call");
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.task("cli_log", "MIT for Mandate Call");
    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.task("cli_log", "List Mandates Call");
    cy.listMandateCallTest(globalState);
  });
});
