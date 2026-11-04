import * as fixtures from "../../../../fixtures/imports";
import State from "../../../../utils/State";
import {
  connectorDetails,
  payment_methods_enabled,
} from "../../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../../configs/Payment/Utils";

let globalState;
let savedState;
let shouldContinue = true;

// A payment on profile A must only be refundable by a JWT scoped to profile A.
// Runs on a throwaway user + merchant so it has a real profile-scoped JWT.
describe("Cross-profile refunds", () => {
  before("set up a throwaway merchant with two profiles", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      savedState = Cypress._.cloneDeep(globalState.data);

      cy.signupUserWithMerchant("CypressCrossProfileRefund", globalState);
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "otherProfile"
      );
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("delete the throwaway merchant and restore global state", () => {
    if (!savedState) {
      return;
    }
    if (globalState.get("merchantId") !== savedState.merchantId) {
      cy.merchantDeleteCall(globalState);
    }
    cy.then(() => {
      globalState.data = savedState;
      cy.task("setGlobalState", globalState.data);
    });
  });

  context("Refund of a profile A payment from profile B", () => {
    it("create and confirm a payment on profile A", () => {
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

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("profile B JWT cannot refund the profile A payment", () => {
      cy.switchProfileCall(globalState.get("otherProfileId"), globalState);
      cy.jwtRefundCallTest(
        fixtures.refundBody,
        connectorDetails.cross_profile_refund.RefundFromOtherProfile,
        globalState
      );
    });

    it("merchant api-key refunds the payment regardless of profile", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("profile B JWT cannot retrieve the profile A refund", () => {
      cy.jwtRefundCallTest(
        fixtures.refundBody,
        connectorDetails.cross_profile_refund.RetrieveRefundFromOtherProfile,
        globalState,
        globalState.get("refundId")
      );
    });

    it("profile A JWT retrieves its own refund", () => {
      cy.switchProfileCall(globalState.get("profileId"), globalState);
      cy.jwtRefundCallTest(
        fixtures.refundBody,
        connectorDetails.cross_profile_refund.RetrieveRefundFromOwnProfile,
        globalState,
        globalState.get("refundId")
      );
    });
  });

  context("Refund of a profile A payment from profile A", () => {
    it("create and confirm a payment on profile A", () => {
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

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("profile A JWT refunds its own payment", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      if (!utils.should_continue_further(data)) {
        this.skip();
      }
      cy.jwtRefundCallTest(fixtures.refundBody, data, globalState);
    });
  });
});
