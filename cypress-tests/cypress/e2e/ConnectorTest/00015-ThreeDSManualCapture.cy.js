import createPaymentBody from "../../fixtures/create-payment-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";
import captureBody from "../../fixtures/capture-flow-body.json";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {

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

    context("Card - ThreeDS Manual Full Capture payment flow test", () => {

        context("payment Create and Confirm", () => {

            it("create-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.createPaymentIntentTest(createPaymentBody, det, "three_ds", "manual", globalState);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.confirmCallTest(confirmBody, det, true, globalState);
            });

            it("Handle redirection", () => {
                let expected_redirection = confirmBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.captureCallTest(captureBody, 6500, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });

        context("Payment Create+Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "three_ds", "manual", globalState);
            });

            it("Handle redirection", () => {
                let expected_redirection = createConfirmPaymentBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })


            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.captureCallTest(captureBody, 6540, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });


    });

    context("Card - ThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {

        context("payment Create and Payment Confirm", () => {

            it("create-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.createPaymentIntentTest(createPaymentBody, det, "three_ds", "manual", globalState);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.confirmCallTest(confirmBody, det, true, globalState);
            });

            it("Handle redirection", () => {
                let expected_redirection = confirmBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.captureCallTest(captureBody, 100, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });

        context("payment + Confirm", () => {
            it("create+confirm-payment-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "three_ds", "manual", globalState);
            });

            it("Handle redirection", () => {
                let expected_redirection = createConfirmPaymentBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
                cy.captureCallTest(captureBody, 5000, det.paymentSuccessfulStatus, globalState);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });


    });
});