import captureBody from "../../fixtures/capture-flow-body.json";
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

    context("Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 7000, true, "automatic", "new_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });
    });

    context("Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 7000, true, "manual", "new_mandate", globalState);
        });

        it("cit-capture-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.captureCallTest(captureBody, req_data, res_data, 7000, globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "manual", globalState);
        });

        it("mit-capture-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.captureCallTest(captureBody, req_data, res_data, 7000, globalState);
        });

        it("list-mandate-call-test", () => {
            cy.listMandateCallTest(globalState);
        });
    });

    context.skip("Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUse3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 6500, true, "automatic", "new_mandate", globalState);
        });

        it("cit-capture-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUse3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("list-mandate-call-test", () => {
            cy.listMandateCallTest(globalState);
        });
    });
});