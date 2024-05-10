import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import voidBody from "../../fixtures/void-payment-body.json";
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

    context("Card - void payment in Requires_capture state flow test", () => {
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

        it("void-call-test", () => {
            cy.voidCallTest(voidBody, globalState);
        });
    });

    context("Card - void payment in Requires_payment_method state flow test", () => {
        it("create-payment-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("void-call-test", () => {
            cy.voidCallTest(voidBody, globalState);
        });
    });

    context("Card - void payment in Requires_payment_method state flow test", () => {
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
            cy.confirmCallTest(confirmBody, req_data, res_data, false, globalState);
        });

        it("void-call-test", () => {
            cy.voidCallTest(voidBody, globalState);
        });
    });
});