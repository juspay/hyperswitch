import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import voidBody from "../../fixtures/void-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";

let globalState;

describe("Card - NoThreeDS Manual payment void flow test", () => {

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

    context("Card - void payment in requires_capture state flow test", () => {
        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DSManual"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DSManual"];
            console.log("det -> " + det.card);
            cy.confirmCallTest(confirmBody, det, true, globalState);
        });

        it("void-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DSManual"];
            cy.voidCallTest(voidBody, det, globalState);
        });
    });

    context("Card - void payment in requires_payment_method state flow with auto confirm test", () => {
        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("void-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            cy.voidCallTest(voidBody, det, globalState);
        });
    });

    context("Card - void payment in requires_payment_method state flow test", () => {
        it("create-payment-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DSManual"];
            cy.createPaymentIntentTest(createPaymentBody, det, "no_three_ds", "manual", globalState);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.confirmCallTest(confirmBody, det, false, globalState);
        });

        it("void-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DSManual"];
            cy.voidCallTest(voidBody, det, globalState);
        });
    });
});