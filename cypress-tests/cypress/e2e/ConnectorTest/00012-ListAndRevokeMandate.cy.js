import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import mitConfirmBody from "../../fixtures/create-mandate-mit.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import * as utils from "../ConnectorUtils/utils";

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
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 7000, true, "automatic", "new_mandate", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
