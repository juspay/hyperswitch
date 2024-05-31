import createPaymentBody from "../../fixtures/create-payment-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";
import captureBody from "../../fixtures/capture-flow-body.json";
import * as utils from "../ConnectorUtils/utils";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {

  
    before("seed global state", () => {
  
      cy.task('getGlobalState').then((state) => {
        globalState = new State(state);
        console.log("seeding globalState -> " + JSON.stringify(globalState));
      })
  
    })

    afterEach("flush global state", () => {
      console.log("flushing globalState -> " + JSON.stringify(globalState));
      cy.task('setGlobalState', globalState.data);
    })

    context("Card - ThreeDS Manual Full Capture payment flow test", () => {
        context("payment Create and Confirm", () => {
            let should_continue = true; // variable that will be used to skip tests if a previous test fails

            beforeEach(function () { 
                if(!should_continue) {
                    this.skip();
                }
            });

            it("create-payment-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "three_ds", "manual", globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("Handle redirection", () => {
                let expected_redirection = confirmBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });

        context("Payment Create+Confirm", () => {
            let should_continue = true; // variable that will be used to skip tests if a previous test fails

            beforeEach(function () { 
                if(!should_continue) {
                    this.skip();
                }
            });

            it("create+confirm-payment-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "three_ds", "manual", globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("Handle redirection", () => {
                let expected_redirection = createConfirmPaymentBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })


            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });


    });

    context("Card - ThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {
        context("payment Create and Payment Confirm", () => {
            let should_continue = true; // variable that will be used to skip tests if a previous test fails

            beforeEach(function () { 
                if(!should_continue) {
                    this.skip();
                }
            });

            it("create-payment-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "three_ds", "manual", globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("payment_methods-call-test", () => {
                cy.paymentMethodsCallTest(globalState);
            });

            it("confirm-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("Handle redirection", () => {
                let expected_redirection = confirmBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });
        });

        context("payment + Confirm", () => {
            let should_continue = true; // variable that will be used to skip tests if a previous test fails

            beforeEach(function () { 
                if(!should_continue) {
                    this.skip();
                }
            });

            it("create+confirm-payment-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "three_ds", "manual", globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("Handle redirection", () => {
                let expected_redirection = createConfirmPaymentBody["return_url"];
                cy.handleRedirection(globalState, expected_redirection);
              })

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

            it("capture-call-test", () => {
                let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
                if(should_continue) should_continue = utils.should_continue_further(res_data);
            });

            it("retrieve-payment-call-test", () => {
                cy.retrievePaymentCallTest(globalState);
            });

        });


    });
});
