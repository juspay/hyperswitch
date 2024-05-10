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
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
        });
    });

    context("Card - Partial Refund flow test for No-3DS", () => {

        it("create-payment-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
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
            console.log("in_retrieve_call");
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
        });
    });

    context("Fully Refund Card-NoThreeDS payment flow test Create+Confirm", () => {

        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
        });

    });

    context("Partially Refund Card-NoThreeDS payment flow test Create+Confirm", () => {

        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
        });

    });

    context("Card - Full Refund for fully captured No-3DS payment", () => {

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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
        });
    });

    context("Card - Partial Refund for fully captured No-3DS payment", () => {

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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
        });
        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
        });
        it("list-refund-call-test", () => {
            cy.listRefundCallTest(listRefundCall, globalState);
        });
    });

    context("Card - Full Refund for partially captured No-3DS payment", () => {

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
            console.log("det -> " + data.card);
            cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 100, globalState);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
        });
    });

    context("Card - partial Refund for partially captured No-3DS payment", () => {

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
            console.log("det -> " + data.card);
            cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 100, globalState);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
        });
    });

    context("Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test", () => {

        it("Confirm No 3DS CIT", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DS"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + req_data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 7000, true, "automatic", "new_mandate", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("Confirm No 3DS MIT", () => {
            cy.mitForMandatesCallTest(mitConfirmBody, 7000, true, "automatic", globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 7000, globalState);
        });

        it("sync-refund-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.syncRefundCallTest(req_data, res_data, globalState);
        });
    });

});
