import captureBody from "../../fixtures/capture-flow-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";

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
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });

        context("Payment Create+Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });


    });

    context("Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {

        context("payment Create and Payment Confirm", () => {

            it("create-payment-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });

        context("payment + Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                console.log("confirm -> " + globalState.get("connectorId"));
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                console.log("det -> " + data.card);
                cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });


    });
});