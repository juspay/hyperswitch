import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Superposition Config Tests (Extended Card BIN and Requires CVV)", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = globalState.get("connectorId");
      specShouldSkip = utils.shouldIncludeConnector(
        connectorId,
        utils.CONNECTOR_LISTS.INCLUDE.REQUIRES_CVV
      );
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup configs + flush global state", () => {
    cy.deleteExtendedCardBinConfig(globalState);
    cy.deleteSuperpositionConfig(globalState, {
      provider_merchant_id: globalState.get("merchantId"),
      processor_merchant_id: globalState.get("merchantId"),
    });
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Enable Extended Card BIN via superposition (profile_id), verify card_extended_bin is populated",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Enable extended_card_bin via superposition (profile_id)", () => {
        cy.setExtendedCardBinConfig(globalState, true);
      });

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment and verify card_extended_bin is populated", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.retrievePaymentCallTest({ globalState, data }).then((response) => {
          expect(
            response.body.payment_method_data.card.card_extended_bin,
            "card_extended_bin should be populated"
          ).to.not.be.null;
          expect(
            response.body.payment_method_data.card.card_extended_bin,
            "card_extended_bin should be first 8 digits of card number"
          ).to.equal("42424242");
        });
      });
    }
  );

  context(
    "Disable Extended Card BIN via superposition, verify card_extended_bin is null",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Disable extended_card_bin via superposition (profile_id)", () => {
        cy.setExtendedCardBinConfig(globalState, false);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment and verify card_extended_bin is null", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.retrievePaymentCallTest({ globalState, data }).then((response) => {
          expect(
            response.body.payment_method_data.card.card_extended_bin,
            "card_extended_bin should be null when config is disabled"
          ).to.be.null;
        });
      });
    }
  );

  context(
    "Delete superposition config, verify card_extended_bin reverts to default (null)",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Delete extended_card_bin config", () => {
        cy.deleteExtendedCardBinConfig(globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm Payment with card (No3DS auto capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment and verify card_extended_bin is null (default after cleanup)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendedCardBin"];

        cy.retrievePaymentCallTest({ globalState, data }).then((response) => {
          expect(
            response.body.payment_method_data.card.card_extended_bin,
            "card_extended_bin should be null after config deletion (reverts to default)"
          ).to.be.null;
        });
      });
    }
  );

  context(
    "Set requires_cvv=true via superposition — verify on-session saved card requires CVV",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Set requires_cvv=true via superposition", () => {
        cy.setSuperpositionConfig(globalState, "requires_cvv", true, {
          provider_merchant_id: globalState.get("merchantId"),
          processor_merchant_id: globalState.get("merchantId"),
        });
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm — save card on_session with CVV", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOnSession"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOnSession"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      it("List Customer PM by client secret — verify requires_cvv=true", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVListPMOnSession"];
        cy.listCustomerPMByClientSecret(globalState, data);
      });

      it("Create Payment Intent for saved card use", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Save Card Confirm with CVV — expect success", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVSavedCardWithCVV"];
        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
      });
    }
  );

  context(
    "Set requires_cvv=false via superposition — verify off-session saved card succeeds without CVV",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Set requires_cvv=false via superposition", () => {
        cy.setSuperpositionConfig(globalState, "requires_cvv", false, {
          provider_merchant_id: globalState.get("merchantId"),
          processor_merchant_id: globalState.get("merchantId"),
        });
      });

      it("Create Payment Intent (off_session)", () => {
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
      });

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm — save card off_session (no CVV required)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOffSessionMandate"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("List Customer PM by client secret — verify requires_cvv=false", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVListPMOffSession"];
        cy.listCustomerPMByClientSecret(globalState, data);
      });

      it("Create Payment Intent for saved card use (off_session)", () => {
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
      });

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Save Card Confirm without CVV — expect success", () => {
        const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVFalseSavedCardWithoutCVV"];
        cy.saveCardConfirmCallTest(saveCardBody, data, globalState);
      });
    }
  );

  context(
    "Delete requires_cvv config, verify default behavior reverts to true",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Delete requires_cvv config via superposition", () => {
        cy.deleteSuperpositionConfig(globalState, {
          provider_merchant_id: globalState.get("merchantId"),
          processor_merchant_id: globalState.get("merchantId"),
        });
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Create Payment Intent", () => {
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

      it("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("Confirm — save card on_session with CVV", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVOnSession"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("List Customer PM — verify requires_cvv reverts to default (true)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RequiresCVVListPMOnSession"];
        cy.listCustomerPMByClientSecret(globalState, data);
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
