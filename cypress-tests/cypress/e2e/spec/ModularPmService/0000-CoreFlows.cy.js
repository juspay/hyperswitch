import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;

describe("Core flows", () => {
  context("Modular Payment Method core flows", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("merchant create call", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("API key create call", () => {
          cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });
    
    it("Connector create call", () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState
          );
    });


    it("Modular PM Service - Customer create call", () => {
      cy.customerCreateCall(globalState, fixtures.customerCreate);
    });

    it("Modular PM Service - Merchant Config create call", () => {
        cy.merchantConfigCall(globalState, fixtures.merchantConfig);  
    });

    it("Modular PM Service - Organization Config create call", () => {
        cy.orgConfigCreateCall(globalState, fixtures.orgConfig);
    });
    it("Modular PM Service - Payment Method Create call", () => {
        cy.paymentMethodCreateCall(globalState, fixtures.paymentMethodCreate);
    });

    it("Modular PM Service - Payments call with pm_id", () => {
        cy.paymentWithSavedPMCall(globalState, fixtures.modularPmServicePaymentsCall);
    });

    it("Modular PM Service - Update Payment Method call", () => {
        cy.updateSavedPMCall(globalState, fixtures.paymentMethodUpdate);
    });

    it("Modular PM Service - Payment Method List call", () => {
        cy.listSavedPMCall(globalState);
    });

    it("Modular PM Service - Payment Method Session Create call", () => {
        cy.pmSessionCreateCall(globalState, fixtures.paymentMethodSessionCreate);
    });

    it("Modular PM Service - Payment Method Session Retrieve call", () => {
        cy.pmSessionRetrieveCall(globalState);
    });

    it("Modular PM Service - Payment Method Session List call", () => {
        cy.pmSessionListPMCall(globalState);
    });

    it("Modular PM Service - Payment Method Session Update call", () => {
        cy.pmSessionUpdatePMCall(globalState, fixtures.paymentMethodSessionUpadte);
    });

    it("Modular PM Service - Payment Method Session Confirm call", () => {
        cy.pmSessionConfirmCall(globalState, fixtures.paymentMethodSessionConfirm);
    });

    it("Modular PM Service - Get Payment Method from session token call", () => {
        cy.getPMFromTokenCall(globalState);
    });

    it("Modular PM Service - Payments call with pm_token", () => {
        cy.paymentWithSavedPMCall(globalState, fixtures.modularPmServicePaymentsCall,true);
    });

  });
});   