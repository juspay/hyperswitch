import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import mitConfirmBody from "../../fixtures/create-mandate-mit.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";

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

    context("Card - NoThreeDS Create + Confirm Automatic CIT and Single use MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateSingleUseNo3DS"];
            cy.citForMandatesCallTest(citConfirmBody, 0, det, true, "automatic", "setup_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });
    });
    context("Card - NoThreeDS Create + Confirm Automatic CIT and Multi use MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateSingleUseNo3DS"];
            cy.citForMandatesCallTest(citConfirmBody, 0, det, true, "automatic", "setup_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });
        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });
    });

});