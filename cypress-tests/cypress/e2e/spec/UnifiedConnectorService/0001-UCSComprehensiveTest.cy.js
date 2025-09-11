import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { cardCreditEnabled } from "../../configs/PaymentMethodList/Commons";

let globalState;

describe("UCS Comprehensive Test", () => {
  before("Initialize and Setup", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = Cypress.env("CYPRESS_CONNECTOR");
      if (
        utils.shouldIncludeConnector(
          connectorId,
          utils.CONNECTOR_LISTS.INCLUDE.UCS_CONNECTORS
        )
      ) {
        cy.log(`Skipping UCS tests - connector not supported: ${connectorId}`);
        this.skip();
      }
      cy.log(`Running UCS tests for: ${connectorId}`);
    });
  });

  after("UCS Cleanup", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("0001-UCS Setup", () => {
    it("merchant-create-call-test", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("connector-create-call-test", () => {
      cy.createNamedConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        cardCreditEnabled,
        globalState,
        "authorizedotnet",
        "authorizedotnet_default"
      );
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("setup-ucs-configs", () => {
      const connectorId = globalState.get("connectorId");
      cy.setupUCSConfigs(globalState, connectorId);
    });
  });

  context("00004-NoThreeDSAutoCapture", () => {
    it("create-payment-call-test", () => {
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
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.retrievePaymentCallTest(globalState, data);
    });

    it("create+confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test-2", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("00005-ThreeDSAutoCapture", () => {
    it("create-payment-call-test", () => {
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
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });
  });

  context("00006-NoThreeDSManualCapture", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];
      cy.captureCallTest(fixtures.captureBody, data, globalState);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];
      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("00007-VoidPayment", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("void-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Void"];
      cy.voidCallTest(fixtures.voidBody, data, globalState);
    });
  });

  context("00008-SyncPayment", () => {
    it("create-payment-call-test", () => {
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
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("sync-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncPayment"];
      cy.syncCallTest(data, globalState);
    });
  });

  context("00009-RefundPayment", () => {
    it("create-payment-call-test", () => {
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
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];
      cy.refundCallTest(fixtures.refundBody, data, globalState);
    });
  });

  context("00010-SyncRefund", () => {
    it("create-payment-call-test", () => {
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
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];
      cy.refundCallTest(fixtures.refundBody, data, globalState);
    });

    it("sync-refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];
      cy.syncRefundCallTest(data, globalState);
    });
  });

  context("00011-CreateSingleuseMandate", () => {
    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateSingleUseNo3DSAutoCapture"];
      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );
    });

    it("Confirm No 3DS MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];
      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );
    });
  });

  context("00012-CreateMultiuseMandate", () => {
    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateMultiUseNo3DSAutoCapture"];
      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );
    });

    it("Confirm No 3DS MIT 1", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];
      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );
    });

    it("Confirm No 3DS MIT 2", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];
      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );
    });
  });

  context("00013-ListAndRevokeMandate", () => {
    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateSingleUseNo3DSAutoCapture"];
      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );
    });

    it("list-mandate-call-test", () => {
      cy.listMandateCallTest(globalState);
    });

    it("revoke-mandate-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateRevoke"];
      cy.revokeMandateCallTest(fixtures.revokeMandateBody, data, globalState);
    });
  });

  context("00014-SaveCardFlow", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCapture"];
      cy.createConfirmPaymentTest(
        fixtures.saveCardConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("list-payment-method-call-test", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("payment-method-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SavedCardConfirm"];
      cy.createConfirmWithSavedPMTest(
        fixtures.createConfirmPaymentBody,
        data,
        globalState
      );
    });
  });

  context("00015-ZeroAuthMandate", () => {
    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Confirm No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ZeroAuthMandate"];
      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        0,
        true,
        "automatic",
        "setup_mandate",
        globalState
      );
    });

    it("Confirm No 3DS MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];
      cy.mitUsingPMId(
        fixtures.pmIdConfirmBody,
        data,
        7000,
        true,
        "automatic",
        globalState
      );
    });
  });

  context("00016-ThreeDSManualCapture", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];
      cy.captureCallTest(fixtures.captureBody, data, globalState);
    });
  });

  context("00019-UCS Cleanup", () => {
    it("cleanup-ucs-configs", () => {
      const connectorId = globalState.get("connectorId");
      cy.cleanupUCSConfigs(globalState, connectorId);
    });
  });
});
