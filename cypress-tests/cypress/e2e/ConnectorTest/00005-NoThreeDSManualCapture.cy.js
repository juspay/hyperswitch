import createPaymentBody from "../../fixtures/create-payment-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";
import captureBody from "../../fixtures/capture-flow-body.json";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {

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

    context("Card - NoThreeDS Manual Full Capture payment flow test", () => {

        it("create-payment-call-test", () => {
            cy.createPaymentIntentTest(createPaymentBody, "EUR", "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.confirmCallTest(confirmBody, det, true, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 6500, det.successfulStates, globalState);
        });
    });

    context("Card - NoThreeDS Manual Partial Capture payment flow test", () => {

        it("create-payment-call-test", () => {
            cy.createPaymentIntentTest(createPaymentBody, "EUR", "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.confirmCallTest(confirmBody, det, true, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.captureCallTest(captureBody, 100, det.successfulStates, globalState);
        });
    });
});