import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
    it("Create Customer -> Create Payment Method -> List PM for Customer", () => {
      let shouldContinue = true;

      step("Create Customer", shouldContinue, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      step("Create Payment Method", shouldContinue, () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("List PM for Customer", shouldContinue, () => {
        cy.listCustomerPMCallTest(globalState);
      });
    });
  });

  context("Set default payment method", () => {
    it("List PM for Customer -> Create Payment Method -> Create Payment Intent -> Confirm Payment -> List PM for Customer -> Set Default Payment Method", () => {
      let shouldContinue = true;

      step("List PM for Customer", shouldContinue, () => {
        cy.listCustomerPMCallTest(globalState);
      });

      step("Create Payment Method", shouldContinue, () => {
        const pmData = getConnectorDetails("commons")["card_pm"][
          "PaymentMethod"
        ];
        cy.createPaymentMethodTest(globalState, pmData);
        if (!utils.should_continue_further(pmData)) {
          shouldContinue = false;
        }
      });

      step("Create Payment Intent", shouldContinue, () => {
        const intentData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          intentData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(intentData)) {
          shouldContinue = false;
        }
      });

      step("Confirm Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SaveCardUseNo3DSAutoCaptureOffSession"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("List PM for Customer", shouldContinue, () => {
        cy.listCustomerPMCallTest(globalState);
      });

      step("Set Default Payment Method", shouldContinue, () => {
        cy.setDefaultPaymentMethodTest(globalState);
      });
    });
  });

  context("Delete payment method for customer", () => {
    it("Create Customer -> Create Payment Method -> List PM for Customer -> Delete Payment Method for Customer", () => {
      let shouldContinue = true;

      step("Create Customer", shouldContinue, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      step("Create Payment Method", shouldContinue, () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("List PM for Customer", shouldContinue, () => {
        cy.listCustomerPMCallTest(globalState);
      });

      step("Delete Payment Method for Customer", shouldContinue, () => {
        cy.deletePaymentMethodTest(globalState);
      });
    });
  });

  context("'Last Used' off-session token payments", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    context("No 3DS save card", () => {
      it("Create Customer -> Create+Confirm Payment (No 3DS Off Session) -> List PM for Customer", () => {
        step("Create Customer", shouldContinue, () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        step(
          "Create+Confirm Payment (No 3DS Off Session)",
          shouldContinue,
          () => {
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
          }
        );

        step("List PM for Customer", shouldContinue, () => {
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });

    context("3DS save card", () => {
      it("Create+Confirm Payment (3DS Off Session) -> Handle Redirection -> List PM for Customer", () => {
        step(
          "Create+Confirm Payment (3DS Off Session)",
          shouldContinue,
          () => {
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
          }
        );

        step("Handle Redirection", shouldContinue, () => {
          const expectedRedirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expectedRedirection);
        });

        step("List PM for Customer", shouldContinue, () => {
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });

    context("3DS save card with token", () => {
      it("Create Payment Intent -> Confirm Save Card Payment -> Handle Redirection -> List PM for Customer", () => {
        step("Create Payment Intent", shouldContinue, () => {
          const intentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntent"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            intentData,
            "three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(intentData)) {
            shouldContinue = false;
          }
        });

        step("Confirm Save Card Payment", shouldContinue, () => {
          const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          const newData = {
            ...confirmData,
            Response: {
              ...confirmData.Response,
              body: {
                ...confirmData.Response.body,
                status: "requires_customer_action",
              },
            },
          };
          cy.saveCardConfirmCallTest(saveCardBody, newData, globalState);
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Handle Redirection", shouldContinue, () => {
          const expectedRedirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expectedRedirection);
        });

        step("List PM for Customer", shouldContinue, () => {
          cy.listCustomerPMCallTest(globalState, 1);
        });
      });
    });

    context("No 3DS save card with token", () => {
      it("Create Payment Intent -> Confirm Save Card Payment -> List PM for Customer", () => {
        step("Create Payment Intent", shouldContinue, () => {
          const intentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntent"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            intentData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(intentData)) {
            shouldContinue = false;
          }
        });

        step("Confirm Save Card Payment", shouldContinue, () => {
          const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState);
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("List PM for Customer", shouldContinue, () => {
          cy.listCustomerPMCallTest(globalState);
        });
      });
    });
  });
});
