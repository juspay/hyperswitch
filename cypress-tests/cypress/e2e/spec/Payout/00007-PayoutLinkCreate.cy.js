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
      ]["PayoutLinkBase"];
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
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        description: "Test with custom theme",
        payout_link_config: {
          ...data.Request.payout_link_config,
          theme: "#FF6B35",
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-logo-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        currency: "EUR",
        description: "Test with merchant logo",
        payout_link_config: {
          ...data.Request.payout_link_config,
          logo: "https://example.com/logo.png",
          merchant_name: "Test Merchant Inc",
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-accordion-layout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        currency: "GBP",
        description: "Test with accordion layout",
        payout_link_config: {
          ...data.Request.payout_link_config,
          sdk_layout: "accordion",
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-tabs-layout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        currency: "EUR",
        description: "Test with tabs layout",
        payout_link_config: {
          ...data.Request.payout_link_config,
          sdk_layout: "tabs",
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payout-link-with-custom-id-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        description: "Test custom payout link id",
        payout_link_config: {
          ...data.Request.payout_link_config,
          payout_link_id: "custom_payout_link_123",
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
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
      ]["PayoutLinkBase"];
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
      ]["PayoutLinkBase"];
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
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        description: "Test Payout Link Card Payment",
        payout_link_config: {
          ...data.Request.payout_link_config,
          enabled_payment_methods: ["card"],
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Visit payout page and submit card details", () => {
      const cardData = {
        card_number: "4242424242424242",
        card_exp_month: "12",
        card_exp_year: "35",
        card_cvc: "123",
      };
      cy.handlePayoutLinkCardRedirection(globalState, cardData, "success");
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

    it("create-payout-link-for-invalid-card-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        description: "Test Payout Link Invalid Card",
        payout_link_config: {
          ...data.Request.payout_link_config,
          enabled_payment_methods: ["card"],
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("update-payout-with-invalid-card-test", () => {
      const cardData = {
        card_number: "4000000000000002",
        card_exp_month: "12",
        card_exp_year: "35",
        card_cvc: "123",
      };
      const errorData = {
        Response: {
          status: 400,
          body: {},
        },
      };
      cy.updatePayoutCallTest(
        { payout_method_data: { card: cardData } },
        errorData,
        false,
        globalState
      );
    });

    it("retrieve-payout-after-invalid-card-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });

    it("create-payout-link-for-expired-card-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        description: "Test Payout Link Expired Card",
        payout_link_config: {
          ...data.Request.payout_link_config,
          enabled_payment_methods: ["card"],
        },
      };
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        { ...data, Request: reqData },
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("update-payout-with-expired-card-test", () => {
      const cardData = {
        card_number: "4000000000000069",
        card_exp_month: "12",
        card_exp_year: "35",
        card_cvc: "123",
      };
      const errorData = {
        Response: {
          status: 400,
          body: {},
        },
      };
      cy.updatePayoutCallTest(
        { payout_method_data: { card: cardData } },
        errorData,
        false,
        globalState
      );
    });

    it("retrieve-payout-after-expired-card-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
