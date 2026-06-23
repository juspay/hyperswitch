import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Payment Link", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.PAYMENT_LINK_CARD
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

  beforeEach(function () {
    if (
      Cypress.browser.isHeadless &&
      (this.currentTest.title.startsWith("Visit payment page") ||
        this.currentTest.title.startsWith("Retrieve Payment after"))
    ) {
      cy.log(
        "Skipping payment link UI test in headless mode — SDK requires headed browser"
      );
      this.skip();
    }
  });

  context("Payment Link - Basic creation and retrieval", () => {
    it("Create Payment Intent with Payment Link", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkBasic"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Initiate Payment Link (Customer-Facing)", () => {
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Retrieve Payment Link (Merchant API)", () => {
      cy.retrievePaymentLinkTest({}, globalState);
    });

    it("List Payment Links", () => {
      cy.listPaymentLinksTest({}, globalState);
    });
  });

  context("Payment Link - Create and Pay with Card", () => {
    it("Create Payment Intent with Payment Link", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkBasic"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Visit payment page and confirm with card (UI)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkCardPayment"];
      cy.handlePaymentLinkCardRedirection(globalState, data.CardData);
    });

    it("Retrieve Payment after card payment", () => {
      cy.retrievePaymentCallTest({
        globalState,
        data: {
          Configs: {
            skipConnectorIdAssertion: true,
            skipBillingAssertion: true,
          },
        },
      });
    });
  });

  context("Payment Link - Configuration Variations", () => {
    it("Create Payment Link with custom theme color", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkWithTheme"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      cy.retrievePaymentLinkTest({}, globalState);
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Create Payment Link with merchant logo", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkWithLogo"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      cy.retrievePaymentLinkTest({}, globalState);
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Create Payment Link with accordion SDK layout", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkWithSdkLayout"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      cy.retrievePaymentLinkTest({}, globalState);
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Create Payment Link with tabs SDK layout", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkTabsLayout"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      cy.retrievePaymentLinkTest({}, globalState);
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Visit payment page with tabs layout and confirm with card", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLinkTabsLayout"];
      cy.handlePaymentLinkCardRedirection(globalState, data.CardData);
    });
  });

  context("Payment Link - 3DS Card Flow", () => {
    it("Create Payment Intent with Payment Link for 3DS card", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLink3DSCard"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("Visit payment page and confirm with 3DS card", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "payment_link_pm"
      ]["PaymentLink3DSCard"];
      cy.handlePaymentLinkCardRedirection(globalState, data.CardData);
    });

    it("Retrieve Payment after 3DS card payment", () => {
      cy.retrievePaymentCallTest({
        globalState,
        data: {
          Configs: {
            skipConnectorIdAssertion: true,
            skipBillingAssertion: true,
          },
        },
      });
    });
  });

  context("Payment Link - Edge Cases", () => {
    it("Create Payment Intent without Payment Link - should not have payment_link in response", () => {
      cy.createPaymentWithoutPaymentLinkTest(
        fixtures.createPaymentBody,
        globalState
      );
    });

    it("Retrieve non-existent Payment Link - should return 404", () => {
      cy.retrieveNonExistentPaymentLinkTest(globalState);
    });
  });
});
