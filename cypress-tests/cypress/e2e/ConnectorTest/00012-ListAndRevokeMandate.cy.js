import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import mitConfirmBody from "../../fixtures/create-mandate-mit.json";
import getConnectorDetails from "../ConnectorUtils/utils";

import State from "../../utils/State";

let globalState;

describe("Card - SingleUse Mandates flow test", () => {

    before("seed global state", () => {

        cy.task('getGlobalState').then((state) => {
            globalState = new State(state);
            console.log("seeding globalState -> " + JSON.stringify(globalState));
        })
    })

    after("flush global state", () => {
        console.log("flushing globalState -> " + JSON.stringify(globalState));
        cy.task('setGlobalState', globalState.data);
    })

    context("Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateSingleUseNo3DS"];
            console.log("det -> " + det.card);
            cy.citForMandatesCallTest(citConfirmBody, 7000, det, true, "automatic", "new_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("list-mandate-call-test", () => {
            cy.listMandateCallTest(globalState);
        });

        it("revoke-mandate-call-test", () => {
            cy.revokeMandateCallTest(globalState);
        });

        it("revoke-revoked-mandate-call-test", () => {
            cy.revokeMandateCallTest(globalState);
        });
    });

}); 