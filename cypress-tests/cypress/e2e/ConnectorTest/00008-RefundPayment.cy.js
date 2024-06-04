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
import * as utils from "../ConnectorUtils/utils";

let globalState;

describe("Card - Refund flow test", () => {

    before("seed global state", () => {

        cy.task('getGlobalState').then((state) => {
          globalState = new State(state);
        })
    
      })
    
      afterEach("flush global state", () => {
        cy.task('setGlobalState', globalState.data);
    })

    context("Card - Full Refund flow test for No-3DS", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Card - Partial Refund flow test for No-3DS", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Fully Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
          if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

    });

    context("Partially Refund Card-NoThreeDS payment flow test Create+Confirm", () => {
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
          if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });
    
    context("Card - Full Refund for fully captured No-3DS payment", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Card - Partial Refund for fully captured No-3DS payment", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
        it("list-refund-call-test", () => {
            cy.listRefundCallTest(listRefundCall, globalState);
        });
    });

    context("Card - Full Refund for partially captured No-3DS payment", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 100, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);

        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Card - partial Refund for partially captured No-3DS payment", () => {
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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
            console.log("confirm -> " + globalState.get("connectorId"));
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
            cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
            cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 100, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Card - Full Refund for Create + Confirm Automatic CIT and MIT payment flow test", () => {
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("Confirm No 3DS CIT", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + req_data.card);
            cy.citForMandatesCallTest(citConfirmBody, req_data, res_data, 7000, true, "automatic", "new_mandate", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
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
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

});

    context("Card - Full Refund flow test for 3DS", () => {

        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });
      
        before("seed global state", () => {
      
          cy.task('getGlobalState').then((state) => {
            globalState = new State(state);
          })
      
        })
      
        afterEach("flush global state", () => {
          cy.task('setGlobalState', globalState.data);
        })

        it("create-payment-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
          });
        
          it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
          });
        
          it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Card - Partial Refund flow test for 3DS", () => {

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
            cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
            cy.paymentMethodsCallTest(globalState);
        });

        it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1200, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });

    context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {

        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("create+confirm-payment-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("Handle redirection", () => {
            let expected_redirection = confirmBody["return_url"];
            cy.handleRedirection(globalState, expected_redirection);
          })
    
          it("retrieve-payment-call-test", () => {  
            cy.retrievePaymentCallTest(globalState);
          });
  
          it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
          });
      
      });

    context("Partially Refund Card-ThreeDS payment flow test Create+Confirm", () => {

        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () { 
            if(!should_continue) {
                this.skip();
            }
        });

        it("create+confirm-payment-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "three_ds", "automatic", globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
        
        it("Handle redirection", () => {
            let expected_redirection = confirmBody["return_url"];
            cy.handleRedirection(globalState, expected_redirection);
          })

         it("retrieve-payment-call-test", () => {  
          cy.retrievePaymentCallTest(globalState);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 3000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        it("sync-refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.syncRefundCallTest(req_data, res_data, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    
    });

    context("Card - Full Refund for fully captured 3DS payment", () => {

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

        it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

    });

    context("Card - Partial Refund for fully captured 3DS payment", () => {

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

        it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 5000, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 1500, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });

        
    });

    context("Card - Full Refund for partially captured 3DS payment", () => {

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

        it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 100, globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });


    });

    context("Card - partial Refund for partially captured 3DS payment", () => {

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

        it("Confirm 3DS", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            console.log("det -> " + data.card);
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

        it("refund-call-test", () => {
            let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
            let req_data = data["Request"];
            let res_data = data["Response"];
            cy.refundCallTest(refundBody, req_data, res_data, 50 , globalState);
            if(should_continue) should_continue = utils.should_continue_further(res_data);
        });
    });
