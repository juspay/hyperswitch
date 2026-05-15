import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

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
      ]["PaymentLinkConfirmCard"];
      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Handle payment page and confirm with card", () => {
      cy.initiatePaymentLinkTest({}, globalState);
    });

    it("Retrieve Payment after card payment", () => {
      cy.retrievePaymentCallTest({
        globalState,
        data: {
          Configs: { skipConnectorIdAssertion: true, skipBillingAssertion: true },
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
