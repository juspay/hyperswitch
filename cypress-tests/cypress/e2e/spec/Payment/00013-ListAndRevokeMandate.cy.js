import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - List and revoke Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete NoThreeDS automatic CIT and MIT then list and revoke mandate", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // List mandate
    cy.listMandateCallTest(globalState);

    // Revoke mandate
    cy.revokeMandateCallTest(globalState);

    // Revoke already revoked mandate
    cy.revokeMandateCallTest(globalState);
  });

  it("should complete zero auth CIT and MIT then list and revoke mandate", () => {
    // Confirm No 3DS CIT with zero auth
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthMandate"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 0, true, "automatic", "setup_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // List mandate
    cy.listMandateCallTest(globalState);

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // List mandate
    cy.listMandateCallTest(globalState);

    // Revoke mandate
    cy.revokeMandateCallTest(globalState);
  });
});