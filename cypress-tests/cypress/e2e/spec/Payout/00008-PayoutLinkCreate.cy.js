import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

const bpWithPayoutLink = {
  payout_link_config: {
    domain_name: "checkout.example.com",
    allowed_domains: ["*.example.com", "trusted-site.com"],
    ui_config: {
      logo: "https://example.com/logo.png",
      merchant_name: "Test Merchant",
      theme: "#4285F4",
    },
    form_layout: "tabs",
    payout_test_mode: true,
  },
};

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

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.PAYOUT_LINK.includes(
            globalState.get("connectorId")
          )
        ) {
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

  after("reset business profile payout_link_config", () => {
    cy.resetBusinessProfilePayoutLinkConfig(globalState);
  });

  beforeEach(function () {
    if (!shouldContinue) {
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
      cy.retrievePayoutCallTest(globalState);
    });

    it("list-payout-links-test", () => {
      cy.listPayoutsTest(globalState);
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
  });

  context("Payout Link - Validation errors", () => {
    it("missing-customer-id-error-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkValidationError"];
      cy.createPayoutWithLinkTest(
        fixtures.createPayoutLinkBody,
        data,
        globalState
      );
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
    });

    it("retrieve-non-existent-payout-link-test", () => {
      cy.retrieveNonExistentPayoutTest(globalState);
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

    it("create-payout-link-with-journey-layout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["PayoutLinkBase"];
      const reqData = {
        ...data.Request,
        currency: "GBP",
        description: "Test with journey layout",
        payout_link_config: {
          ...data.Request.payout_link_config,
          form_layout: "journey",
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
          form_layout: "tabs",
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
          payout_link_id: `custom_payout_link_${Date.now()}`,
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

  context("Payout Link - Bank transfer form submission", () => {
    let shouldContinue = true;

    before("reset business profile payout_link_config", () => {
      cy.resetBusinessProfilePayoutLinkConfig(globalState);
    });

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

    it("retrieve-payout-after-bank-submission-test", function () {
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
      cy.UpdateBusinessProfileTest(
        bpWithPayoutLink,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        globalState
      );
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
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
