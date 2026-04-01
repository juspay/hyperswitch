import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create payment method for customer", () => {
    it("Create customer -> Create Payment Method -> List PM for customer", () => {
      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

        cy.createPaymentMethodTest(globalState, data);
      });

      cy.step("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });
    });
  });

  context("Set default payment method", () => {
    it("List PM for customer -> Create Payment Method -> create-payment-call-test -> confirm-payment-call-test -> List PM for customer -> Set default payment method", () => {
      let shouldContinue = true;

      cy.step("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

        cy.createPaymentMethodTest(globalState, data);
      });

      cy.step("create-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("confirm-payment-call-test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm-payment-call-test");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List PM for customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List PM for customer");
          return;
        }
        cy.listCustomerPMCallTest(globalState);
      });

      cy.step("Set default payment method", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Set default payment method");
          return;
        }
        cy.setDefaultPaymentMethodTest(globalState);
      });
    });
  });

  context("Delete payment method for customer", () => {
    it("Create customer -> Create Payment Method -> List PM for customer -> Delete Payment Method for a customer", () => {
      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);
      });

      cy.step("List PM for customer", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      cy.step("Delete Payment Method for a customer", () => {
        cy.deletePaymentMethodTest(globalState);
      });
    });
  });

  context("'Last Used' off-session token payments", () => {
    let shouldContinue = true;

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create customer", () => {
      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });
    });

    context("Create No 3DS off session save card payment", () => {
      it("create+confirm-payment-call-test -> List PM for customer", () => {
        cy.step("create+confirm-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SaveCardUseNo3DSAutoCaptureOffSession"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List PM for customer", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List PM for customer");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });

    context("Create 3DS off session save card payment", () => {
      it("create+confirm-payment-call-test -> Handle redirection -> List PM for customer", () => {
        cy.step("create+confirm-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SaveCardUse3DSAutoCaptureOffSession"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expectedRedirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expectedRedirection);
        });

        cy.step("List PM for customer", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List PM for customer");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });

    context("Create 3DS off session save card payment with token", () => {
      it("create-payment-call-test -> confirm-save-card-payment-call-test -> Handle redirection -> List PM for customer", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

        cy.step("create-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("confirm-save-card-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: confirm-save-card-payment-call-test"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SaveCardUseNo3DSAutoCapture"];

          const newData = {
            ...data,
            Response: {
              ...data.Response,
              body: {
                ...data.Response.body,
                status: "requires_customer_action",
              },
            },
          };

          cy.saveCardConfirmCallTest(saveCardBody, newData, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expectedRedirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expectedRedirection);
        });

        cy.step("List PM for customer", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List PM for customer");
            return;
          }
          cy.listCustomerPMCallTest(globalState, 1 /* order */);
        });
      });
    });

    context("Create No 3DS off session save card payment with token", () => {
      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create-payment-call-test -> confirm-save-card-payment-call-test -> List PM for customer", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

        cy.step("create-payment-call-test", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("confirm-save-card-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: confirm-save-card-payment-call-test"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SaveCardUseNo3DSAutoCapture"];

          cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List PM for customer", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List PM for customer");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });
  });
});
