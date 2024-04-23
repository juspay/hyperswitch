import createPaymentBody from "../../fixtures/create-payment-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";
import captureBody from "../../fixtures/capture-flow-body.json";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {

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

    context("Card - NoThreeDS Manual Full Capture payment flow test", () => {

        context("payment Create and Confirm", () => {

            it("create-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                cy.createPaymentIntentTest(createPaymentBody, det.currency, "no_three_ds", "manual", globalState);
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
                cy.captureCallTest(captureBody, 6500, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });

        context("Payment Create+Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                console.log("det -> " + det.card);
                cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "no_three_ds", "manual", globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                console.log("det -> " + det.card);
                cy.captureCallTest(captureBody, 6540, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });


    });

    context("Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {

        context("payment Create and Payment Confirm", () => {

            it("create-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                cy.createPaymentIntentTest(createPaymentBody, det.currency, "no_three_ds", "manual", globalState);
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
                cy.captureCallTest(captureBody, 100, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });

        context("payment + Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                console.log("det -> " + det.card);
                cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "no_three_ds", "manual", globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
                console.log("det -> " + det.card);
                cy.captureCallTest(captureBody, 5000, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });


    });
});