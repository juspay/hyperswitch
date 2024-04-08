import createPaymentBody from "../../fixtures/create-payment-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import customerCreateBody from "../../fixtures/create-customer-body.json";
import captureBody from "../../fixtures/capture-flow-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";
let globalState;
describe("Card - NoThreeDS payment flow test", () => {  

    before("seed global state",  () => {
      
      cy.task('getGlobalState').then((state) => {
        // visit non same-origin url https://www.cypress-dx.com
        globalState = new State(state);
        console.log("seeding globalState -> " + JSON.stringify(globalState));
      })
    })
  
    after("flush global state", () => {
      console.log("flushing globalState -> "+ JSON.stringify(globalState));
      cy.task('setGlobalState', globalState.data);
    })

  
    context("Save card for NoThreeDS automatic capture payment- Create+Confirm", () => {
      it("customer-create-call-test", () => {
            cy.createCustomerCallTest(customerCreateBody, globalState); 
      });
  
      it("create+confirm-payment-call-test", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
        cy.createConfirmPaymentTest( createConfirmPaymentBody, det,"no_three_ds", "automatic", globalState);
      });
  
      it("retrieve-payment-call-test", () => {  
        cy.retrievePaymentCallTest(globalState);
      });
       
      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
        cy.createPaymentIntentTest( createPaymentBody, det, "no_three_ds", "automatic", globalState);
      });

      it ("confirm-save-card-payment-call-test", () => {
        let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
        cy.saveCardConfirmCallTest(confirmBody,det,globalState);
      });
      
    });

    context("Save card for NoThreeDS manual full capture payment- Create+Confirm", () => {
        it("customer-create-call-test", () => {
              cy.createCustomerCallTest(customerCreateBody, globalState); 
        });
    
        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, det,"no_three_ds", "automatic", globalState);
        });
    
        it("retrieve-payment-call-test", () => {  
          cy.retrievePaymentCallTest(globalState);
        });
         
        it("retrieve-customerPM-call-test", () => {
          cy.listCustomerPMCallTest(globalState);
        });
  
        it("create-payment-call-test", () => {
          let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
          cy.createPaymentIntentTest( createPaymentBody, det, "no_three_ds", "manual", globalState);
        });

  
        it ("confirm-save-card-payment-call-test", () => {
          let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
          cy.saveCardConfirmCallTest(confirmBody,det,globalState);
        });
        
        it("capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 6500, det.paymentSuccessfulStatus, globalState);
        });       


        
      });

    context("Save card for NoThreeDS manual partial capture payment- Create+Confirm", () => {
        it("customer-create-call-test", () => {
              cy.createCustomerCallTest(customerCreateBody, globalState); 
        });
    
        it("create+confirm-payment-call-test", () => {
          console.log("confirm -> " + globalState.get("connectorId"));
          let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
          cy.createConfirmPaymentTest( createConfirmPaymentBody, det,"no_three_ds", "automatic", globalState);
        });
    
        it("retrieve-payment-call-test", () => {  
          cy.retrievePaymentCallTest(globalState);
        });
         
        it("retrieve-customerPM-call-test", () => {
          cy.listCustomerPMCallTest(globalState);
        });
  
        it("create-payment-call-test", () => {
          let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
          cy.createPaymentIntentTest( createPaymentBody, det, "no_three_ds", "manual", globalState);
        });

  
        it ("confirm-save-card-payment-call-test", () => {
          let det = getConnectorDetails(globalState.get("connectorId"))["SaveCardUseNo3DS"];
          cy.saveCardConfirmCallTest(confirmBody,det,globalState);
        });
        
        it("capture-call-test", () => {
            let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
            console.log("det -> " + det.card);
            cy.captureCallTest(captureBody, 5500, det.paymentSuccessfulStatus, globalState);
        });            
      });
    
});