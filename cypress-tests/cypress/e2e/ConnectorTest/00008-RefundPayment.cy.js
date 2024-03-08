import createPaymentBody from "../../fixtures/create-payment-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import refundBody from "../../fixtures/refund-flow-body.json"
import State from "../../utils/State";

let globalState;

describe("Card - Refund flow test", () => {

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

    context("Card - Full Refund flow test for No-3DS", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det.currency, "no_three_ds", "automatic", globalState);
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

        it("refund-call-test", () => {
            cy.refundCallTest(refundBody, 6500, globalState);
        });
    });

    context("Card - Partial Refund flow test for No-3DS", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det.currency, "no_three_ds", "automatic", globalState);
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

        it("refund-call-test", () => {
            cy.refundCallTest(refundBody, 1200, globalState);
        });

        it("refund-call-test", () => {
            cy.refundCallTest(refundBody, 1200, globalState);
        });
    });

});