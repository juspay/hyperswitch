import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Redirect tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Blik Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Blik");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Blik"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("EPS Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Eps");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Eps"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Eps"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Giropay Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Giropay");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Giropay"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Giropay"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("iDEAL Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Ideal");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Ideal"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Ideal"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Sofort Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Sofort");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Sofort"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Sofort"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Przelewy24 Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Przelewy24");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Przelewy24"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    });
  });

  context("OpenBankingUk Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("OpenBankingUk");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OpenBankingUk"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OpenBankingUk"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("OnlineBankingFpx Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("OnlineBankingFpx");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OnlineBankingFpx"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OnlineBankingFpx"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Interac Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Interac");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Interac"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Interac"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Trustly Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Trustly");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Trustly"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    });
  });

  context("Eft Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Eft");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Eft"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Eft"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context(
    "BancontactCard - MandateSingleUseAutoCapture Create + Confirm flow test",
    () => {
      let shouldContinue = true;

      before(function () {
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.BANK_REDIRECT_BANCONTACT
          )
        ) {
          this.skip();
        }
      });

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create+Confirm BancontactCard mandate auto-capture", () => {
        let shouldContinueInner = true;

        cy.step("Create+Confirm payment with mandate", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["BancontactCard"]["MandateSingleUseAutoCapture"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinueInner = false;
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinueInner) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["BancontactCard"]["MandateSingleUseAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  describe("Bank Redirect Mandate CIT tests", () => {
    before(function () {
      if (
        utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.BANK_REDIRECT_MANDATE
        )
      ) {
        this.skip();
      }
    });

    context("iDEAL - MandateSingleUseAutoCapture CIT", () => {
      it("CIT mandate and retrieve", () => {
        let shouldContinue = true;

        cy.step("CIT for Mandate", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Ideal"]["MandateSingleUseAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            6540,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Ideal"]["MandateSingleUseAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        cy.step("List Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Mandate");
            return;
          }
          cy.listMandateCallTest(globalState);
        });

        cy.step("Revoke Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Revoke Mandate");
            return;
          }
          cy.revokeMandateCallTest(globalState);
        });
      });
    });

    context("OpenBankingUk - MandateSingleUseAutoCapture CIT", () => {
      it("CIT mandate and retrieve", () => {
        let shouldContinue = true;

        cy.step("CIT for Mandate", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["OpenBankingUk"]["MandateSingleUseAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            6540,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["OpenBankingUk"]["MandateSingleUseAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        cy.step("List Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Mandate");
            return;
          }
          cy.listMandateCallTest(globalState);
        });

        cy.step("Revoke Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Revoke Mandate");
            return;
          }
          cy.revokeMandateCallTest(globalState);
        });
      });
    });

    context("Trustly - MandateSingleUseAutoCapture CIT", () => {
      it("CIT mandate and retrieve", () => {
        let shouldContinue = true;

        cy.step("CIT for Mandate", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Trustly"]["MandateSingleUseAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            6540,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Trustly"]["MandateSingleUseAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
        });

        cy.step("List Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Mandate");
            return;
          }
          cy.listMandateCallTest(globalState);
        });

        cy.step("Revoke Mandate", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Revoke Mandate");
            return;
          }
          cy.revokeMandateCallTest(globalState);
        });
      });
    });
  });
});
