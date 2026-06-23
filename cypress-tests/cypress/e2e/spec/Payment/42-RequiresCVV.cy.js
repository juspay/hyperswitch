import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Requires CVV flow test", () => {
  let connector;
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          !utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.REQUIRES_CVV
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "On-session saved card payment requires CVV (requires_cvv=true)",
    () => {
      it("Create Customer -> Create Payment Intent -> Confirm Payment with CVV -> List PMs (requires_cvv=true) -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVPaymentIntent"];
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

        cy.step("Confirm Payment with CVV (on_session)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVOnSession"];
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step(
          "List Customer Payment Methods (verify requires_cvv=true)",
          () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: List Customer Payment Methods"
              );
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["RequiresCVVListPMOnSession"];
            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
              cy.task(
                "cli_log",
                "Skipping step: List Customer Payment Methods (server bug HE_00 workaround)"
              );
              return;
            }
            cy.listCustomerPMByClientSecret(globalState, data);
          }
        );

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVOnSession"];
          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  // BofA sandbox cannot establish off_session mandate - TRIGGER_SKIP makes steps 2-5 no-ops; this context documents the intended off-session mandate behavior
  context("Off-session with mandate skips CVV", () => {
    it("Create Customer -> Create+Confirm Payment (off_session with mandate) -> Retrieve -> List PMs -> Create PI -> Save Card Confirm (without CVV)", () => {
      let shouldContinue = true;

      cy.step("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create and Confirm Payment (off_session with mandate)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create and Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOffSessionMandate"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      cy.step("Retrieve Payment after Confirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOffSessionMandate"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Customer Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Customer Payment Methods");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVListPMOffSession"];
        cy.listCustomerPMByClientSecret(globalState, data);
      });

      cy.step("Create Payment Intent (off_session)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
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

      cy.step("Save Card Confirm Call (without CVV)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Save Card Confirm Call");
          return;
        }
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVSavedCardWithoutCVV"];
        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
      });
    });
  });

  context(
    "Saved card confirm without mandate requires CVV (on_session, BofA workaround for off_session)",
    () => {
      it("Create Customer -> Create+Confirm Payment (on_session save) -> Retrieve -> List PMs -> Create PI (off_session) -> Save Card Confirm (with CVV) -> Retrieve", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create and Confirm Payment (on_session save)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create and Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVOnSession"];
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

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVOnSession"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVListPMOnSession"];
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            cy.task(
              "cli_log",
              "Skipping step: List Customer Payment Methods (server bug HE_00 workaround)"
            );
            return;
          }
          cy.listCustomerPMByClientSecret(globalState, data);
        });

        cy.step("Create Payment Intent (off_session)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
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

        cy.step("Save Card Confirm Call (with CVV)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Save Card Confirm Call");
            return;
          }
          const saveCardBody = Cypress._.cloneDeep(
            fixtures.saveCardConfirmBody
          );
          saveCardBody.card_cvc = "123";
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVSavedCardWithCVV"];
          cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Save Card Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVSavedCardWithCVV"];
          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context("Saved card confirm with CVV (requires_cvv=true)", () => {
    it("Create Customer -> Create+Confirm Payment (save card) -> Retrieve -> List PMs -> Create PI -> Save Card Confirm (with CVV) -> Retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create and Confirm Payment (save card on_session)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create and Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOnSession"];
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

      cy.step("Retrieve Payment after Confirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOnSession"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Customer Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Customer Payment Methods");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVListPMOnSession"];
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
          cy.task(
            "cli_log",
            "Skipping step: List Customer Payment Methods (server bug HE_00 workaround)"
          );
          return;
        }
        cy.listCustomerPMByClientSecret(globalState, data);
      });

      cy.step("Create Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVPaymentIntent"];
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

      cy.step("Save Card Confirm Call (with CVV)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Save Card Confirm Call");
          return;
        }
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        saveCardBody.card_cvc = "123";
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVSavedCardWithCVV"];
        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Save Card Confirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVSavedCardWithCVV"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Saved card confirm without CVV when requires_cvv=false (off_session)",
    () => {
      it("Create Customer -> Create+Confirm Payment (on_session save with requires_cvv=false) -> Retrieve -> List PMs -> Create PI (off_session) -> Save Card Confirm (without CVV) -> Retrieve", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step(
          "Create and Confirm Payment (on_session save with requires_cvv=false)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Create and Confirm Payment");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["RequiresCVVOnSession"];
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

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVOnSession"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVListPMOnSession"];
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
            cy.task(
              "cli_log",
              "Skipping step: List Customer Payment Methods (server bug HE_00 workaround)"
            );
            return;
          }
          cy.listCustomerPMByClientSecret(globalState, data);
        });

        cy.step(
          "Create Payment Intent (off_session with requires_cvv=false)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Create Payment Intent");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["RequiresCVVFalsePaymentIntent"];
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
          }
        );

        cy.step("Save Card Confirm Call (without CVV)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Save Card Confirm Call");
            return;
          }
          const saveCardBody = Cypress._.cloneDeep(
            fixtures.saveCardConfirmBody
          );
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVFalseSavedCardWithoutCVV"];
          cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Save Card Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RequiresCVVFalseSavedCardWithoutCVV"];
          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context("Invalid CVV format validation", () => {
    it("Confirm with short CVV (IR_16)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for short CVV test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVPaymentIntent"];
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

      cy.step("Confirm Payment with short CVV (expect 400 IR_16)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm with short CVV");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVInvalidCVVShort"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });

    it("Confirm with long CVV (IR_16)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for long CVV test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVPaymentIntent"];
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

      cy.step("Confirm Payment with long CVV (expect 400 IR_16)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm with long CVV");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVInvalidCVVLong"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });

    it("Confirm with non-numeric CVV (IR_07)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for non-numeric CVV test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVPaymentIntent"];
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

      cy.step("Confirm Payment with non-numeric CVV (expect 400 IR_07)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm with non-numeric CVV");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVInvalidCVVNonNumeric"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });
  });
});
