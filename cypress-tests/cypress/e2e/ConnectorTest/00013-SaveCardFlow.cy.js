import captureBody from "../../fixtures/capture-flow-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import customerCreateBody from "../../fixtures/create-customer-body.json";
import SaveCardConfirmBody from "../../fixtures/save-card-confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import * as utils from "../ConnectorUtils/utils";
import State from "../../utils/State";
let globalState;

describe("Card - SaveCard payment flow test", () => {

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      globalState = new State(state);
    })
  })

  
  context("Save card for NoThreeDS automatic capture payment- Create+Confirm", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () { 
        if(!should_continue) {
            this.skip();
        }
    });

    it("customer-create-call-test", () => {
          cy.createCustomerCallTest(customerCreateBody, globalState); 
    });

    it("create+confirm-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
      if(should_continue) should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {  
      cy.retrievePaymentCallTest(globalState);
    });
    
    it("retrieve-customerPM-call-test", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
      if(should_continue) should_continue = utils.should_continue_further(res_data);
    });

    it ("confirm-save-card-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.saveCardConfirmCallTest(SaveCardConfirmBody, req_data, res_data, globalState);
      if(should_continue) should_continue = utils.should_continue_further(res_data);
    });
    
  });

    context("Save card for NoThreeDS manual full capture payment- Create+Confirm", () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () { 
          if(!should_continue) {
              this.skip();
          }
      });

      it("customer-create-call-test", () => {
            cy.createCustomerCallTest(customerCreateBody, globalState); 
      });
  
      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });
  
      it("retrieve-payment-call-test", () => {  
        cy.retrievePaymentCallTest(globalState);
      });
      
      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest( createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });


      it ("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(SaveCardConfirmBody, req_data, res_data, globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });

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
    });

    context("Save card for NoThreeDS manual partial capture payment- Create + Confirm", () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () { 
          if(!should_continue) {
              this.skip();
          }
      });

      it("customer-create-call-test", () => {
            cy.createCustomerCallTest(customerCreateBody, globalState); 
      });
  
      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest( createConfirmPaymentBody, req_data, res_data,"no_three_ds", "automatic", globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });
  
      it("retrieve-payment-call-test", () => {  
        cy.retrievePaymentCallTest(globalState);
      });
      
      it("retrieve-customerPM-call-test", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest( createPaymentBody, req_data, res_data, "no_three_ds", "manual", globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });


      it ("confirm-save-card-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.saveCardConfirmCallTest(SaveCardConfirmBody,req_data, res_data,globalState);
        if(should_continue) should_continue = utils.should_continue_further(res_data);
      });
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
    });
    
    
});
