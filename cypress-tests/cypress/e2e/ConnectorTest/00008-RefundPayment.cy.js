import captureBody from "../../fixtures/capture-flow-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import mitConfirmBody from "../../fixtures/create-mandate-mit.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import refundBody from "../../fixtures/refund-flow-body.json";
import listRefundCall from "../../fixtures/list-refund-call-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";

let globalState;

describe("Card - Refund flow test", () => {

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

    context("Card - Full Refund flow test for No-3DS", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "automatic", globalState);
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
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 6500, det, globalState);
        });
    });

    context("Card - Partial Refund flow test for No-3DS", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "automatic", globalState);
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
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 1200, det, globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 1200, det, globalState);
        });
    });

    context("Fully Refund Card-NoThreeDS payment flow test Create+Confirm", () => {

        it("create+confirm-payment-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "no_three_ds", "automatic", globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 6540, det, globalState);
        });

    });

    context("Partially Refund Card-NoThreeDS payment flow test Create+Confirm", () => {

        it("create+confirm-payment-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createConfirmPaymentTest(createConfirmPaymentBody, det, "no_three_ds", "automatic", globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 3000, det, globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 3000, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });

    });

    context("Card - Full Refund for fully captured No-3DS payment", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
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

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 6500, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });
    });

    context("Card - Partial Refund for fully captured No-3DS payment", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
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

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 5000, det, globalState);
        });
        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 500, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });
        it("list-refund-call-test", () => {
            cy.listRefundCallTest(listRefundCall, globalState);
        });
    });

    context("Card - Full Refund for partially captured No-3DS payment", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
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
            cy.captureCallTest(captureBody, 4000, det.paymentSuccessfulStatus, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 4000, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });
    });

    context("Card - partial Refund for partially captured No-3DS payment", () => {

        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
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
            cy.captureCallTest(captureBody, 4000, det.paymentSuccessfulStatus, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 3000, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });
    });

    context("Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["MandateMultiUseNo3DS"];
            console.log("det -> " + det.card);
            cy.citForMandatesCallTest(citConfirmBody, 7000, det, true, "automatic", "new_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.refundCallTest(refundBody, 7000, det, globalState);
        });

        it("sync-refund-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.syncRefundCallTest(det, globalState);
        });
    });

});
