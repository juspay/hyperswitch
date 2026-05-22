import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

describe("Payout Link", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  beforeEach(function () {
    if (
      Cypress.browser.isHeadless &&
      this.currentTest.title.startsWith("Visit payout page")
    ) {
      cy.log(
        "Skipping payout link UI test in headless mode - SDK requires headed browser"
      );
      this.skip();
    }
  });

  context("Payout Link - Basic creation and retrieval", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-link-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBasic"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-link-test", () => {
      cy.retrievePayoutLinkTest({}, globalState);
    });

    it("list-payout-links-test", () => {
      cy.listPayoutLinksTest({}, globalState);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Link - Create without link", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-without-link-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkWithoutLink"];
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        false,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verify-no-payout-link-in-response", () => {
      cy.createPayoutWithoutLinkTest(fixtures.createPayoutBody, globalState);
    });
  });

  context("Payout Link - Validation errors", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("missing-customer-id-error-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkValidationError"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-and-payout-link-conflict-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkConfirmConflict"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-non-existent-payout-link-test", () => {
      cy.retrieveNonExistentPayoutLinkTest(globalState);
    });
  });

  context("Payout Link - Configuration variations", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-with-theme-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkWithTheme"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-logo-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkWithLogo"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-accordion-layout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkWithSdkLayout"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-tabs-layout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkTabsLayout"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-custom-id-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkCustomId"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Payout Link - Hosted page rendering", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-for-page-render-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBasic"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page and verify SDK loads", () => {
      cy.initiatePayoutLinkTest({}, globalState);
    });

    it("retrieve-payout-after-link-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Link - Bank transfer form submission", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-for-bank-transfer-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBankTransfer"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page and submit bank details", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBankTransfer"];
      cy.handlePayoutLinkBankRedirection(globalState, data.BankData, "success");
    });

    it("retrieve-payout-after-bank-submission-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Link - Profile-level configuration", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("update-business-profile-with-payout-link-config-test", () => {
      const profileBody =
        fixtures.businessProfileWithPayoutLink.bpWithPayoutLink;
      const apiKey = globalState.get("apiKey");
      const merchantId = globalState.get("merchantId");
      const profileId =
        globalState.get("profileId") || globalState.get("defaultProfileId");

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": apiKey,
        },
        body: profileBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.profile_id).to.equal(profileId);
        if (response.body.payout_link_config) {
          expect(response.body.payout_link_config).to.have.property(
            "domain_name"
          );
        }
      });
    });

    it("create-payout-link-using-profile-config-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkProfileConfig"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-link-profile-config-test", () => {
      cy.retrievePayoutLinkTest({}, globalState);
    });
  });

  context("Payout Link - Card Payment Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach(function () {
      if (
        Cypress.browser.isHeadless &&
        this.currentTest.title.startsWith("Visit payout page")
      ) {
        cy.log(
          "Skipping payout link card UI test in headless mode - SDK requires headed browser"
        );
        this.skip();
      }
    });

    it("create-payout-link-for-card-payment-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkCardPayment"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page and submit card details", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkCardPayment"];
      cy.handlePayoutLinkCardRedirection(globalState, data.CardData, "success");
    });

    it("retrieve-payout-after-card-submission-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Link - Error Card Scenarios", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach(function () {
      if (
        Cypress.browser.isHeadless &&
        this.currentTest.title.startsWith("Visit payout page")
      ) {
        cy.log(
          "Skipping payout link card UI test in headless mode - SDK requires headed browser"
        );
        this.skip();
      }
    });

    it("create-payout-link-for-invalid-card-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkInvalidCard"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page with invalid card and verify error", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkInvalidCard"];
      cy.handlePayoutLinkCardRedirection(globalState, data.CardData, "error");
    });

    it("retrieve-payout-after-invalid-card-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });

    it("create-payout-link-for-expired-card-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkExpiredCard"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page with expired card and verify error", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkExpiredCard"];
      cy.handlePayoutLinkCardRedirection(globalState, data.CardData, "error");
    });

    it("retrieve-payout-after-expired-card-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
