import createPaymentBody from "../../fixtures/create-payment-body.json";
import captureBody from "../../fixtures/capture-flow-body.json";
import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import mitConfirmBody from "../../fixtures/create-mandate-mit.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";

let globalState;

describe("Card - NoThreeDS CIT payment flow test", () => {

    before("seed global state", () => {

        cy.task('getGlobalState').then((state) => {
            // visit non same-origin url https://www.cypress-dx.com
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
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.citForMandatesCallTest(citConfirmBody, det, true, "automatic", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 6000, true, "automatic", globalState);
        });
        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 6000, true, "automatic", globalState);
        });
    });

    context("Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.citForMandatesCallTest(citConfirmBody, det, true, "manual", globalState);
        });

        it("cit-capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 7000, det.successfulStates, globalState);
        });

        it("Confirm No 3DS MIT 1", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 6000, true, "manual", globalState);
        });

        it("mit-capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 6000, det.successfulStates, globalState);
        });

        it("Confirm No 3DS MIT 2", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 6000, true, "manual", globalState);
        });

        it("mit-capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 6000, det.successfulStates, globalState);
        });
    });

    context.skip("Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUse3DS"];
            console.log("det -> " + det.card);
            cy.citForMandatesCallTest(citConfirmBody, det, true, "automatic", globalState);
        });

        it("cit-capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUse3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 6500, det.successfulStates, globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });
    });
});