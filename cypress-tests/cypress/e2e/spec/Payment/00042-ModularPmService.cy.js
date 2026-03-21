import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;
let shouldContinue;

describe("Modular Payment Method Service", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      shouldContinue = true;
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Modular Payment Method Service", () => {
    it("Modular PM Service - Customer create call", () => {
      cy.customerCreateCall(globalState, fixtures.customerCreate);
    });

    it("Modular PM Service - Merchant Config create call", () => {
      const key = `should_return_raw_payment_method_details_${globalState.get("merchantId")}`;
      cy.setConfigs(globalState, key, "true", "CREATE");
    });

    it("Modular PM Service - Organization Config create call", () => {
      const key = `should_call_pm_modular_service_${globalState.get("organizationId")}`;
      cy.setConfigs(globalState, key, "true", "CREATE");
    });

    it("Modular PM Service - Payment Method Create call", () => {
      cy.step("Payment Method Create", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Create");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "modular_pm"
        ]["PaymentMethodCreate"];

        cy.paymentMethodCreateCall(
          globalState,
          fixtures.paymentMethodCreate,
          data
        );

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });

    it("Modular PM Service - Payments call with pm_id", () => {
      cy.step("Payments with pm_id", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payments with pm_id");
          return;
        }

        const connectorDetails = getConnectorDetails(
          globalState.get("connectorId")
        );
        const data =
          connectorDetails["modular_pm"]["PaymentsWithPmId"] ||
          connectorDetails["card_pm"]["No3DSAutoCapture"];

        cy.paymentWithSavedPMCall(
          globalState,
          fixtures.modularPmServicePaymentsCall,
          false,
          data
        );

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });

    it("Modular PM Service - Update Payment Method call", () => {
      cy.step("Update Payment Method", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Payment Method");
          return;
        }

        cy.updateSavedPMCall(globalState, fixtures.paymentMethodUpdate);
      });
    });

    it("Modular PM Service - Payment Method List call", () => {
      cy.step("Payment Method List", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method List");
          return;
        }

        cy.listSavedPMCall(globalState);
      });
    });

    it("Modular PM Service - Payment Method Session Create call", () => {
      cy.step("Payment Method Session Create", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Session Create");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "modular_pm"
        ]["PaymentMethodSessionCreate"];

        cy.pmSessionCreateCall(
          globalState,
          fixtures.paymentMethodSessionCreate,
          data
        );

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });

    it("Modular PM Service - Payment Method Session Retrieve call", () => {
      cy.step("Payment Method Session Retrieve", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Session Retrieve");
          return;
        }

        cy.pmSessionRetrieveCall(globalState);
      });
    });

    it("Modular PM Service - Payment Method Session List call", () => {
      cy.step("Payment Method Session List", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Session List");
          return;
        }

        cy.pmSessionListPMCall(globalState);
      });
    });

    it("Modular PM Service - Payment Method Session Update call", () => {
      cy.step("Payment Method Session Update", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Session Update");
          return;
        }

        cy.pmSessionUpdatePMCall(
          globalState,
          fixtures.paymentMethodSessionUpdate
        );
      });
    });

    it("Modular PM Service - Payment Method Session Confirm call", () => {
      cy.step("Payment Method Session Confirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Method Session Confirm");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "modular_pm"
        ]["PaymentMethodSessionConfirm"];

        cy.pmSessionConfirmCall(
          globalState,
          fixtures.paymentMethodSessionConfirm,
          data
        );

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });

    it("Modular PM Service - Get Payment Method from session token call", () => {
      cy.step("Get Payment Method from session token", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Get Payment Method from session token"
          );
          return;
        }

        cy.getPMFromTokenCall(globalState);
      });
    });

    it("Modular PM Service - Payments call with pm_token", () => {
      cy.step("Payments with pm_token", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payments with pm_token");
          return;
        }

        const connectorDetails = getConnectorDetails(
          globalState.get("connectorId")
        );
        const data =
          connectorDetails["modular_pm"]["PaymentsWithPmToken"] ||
          connectorDetails["card_pm"]["No3DSAutoCapture"];

        cy.paymentWithSavedPMCall(
          globalState,
          fixtures.modularPmServicePaymentsCall,
          true,
          data
        );

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });
  });
});
