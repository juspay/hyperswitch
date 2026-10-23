import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Fall back to billing name when cardholder name is absent", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.CARD_HOLDER_NAME_BILLING_FALLBACK
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

  context(
    "Card-NoThreeDS payment without card_holder_name falls back to billing name",
    () => {
      it("create-confirm-payment-without-card-holder-name", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardHolderNameFallbackToBilling"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardHolderNameFallbackToBilling"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );

  context(
    "Card-NoThreeDS payment without card_holder_name and without a billing name to fall back to fails",
    () => {
      it("create-confirm-payment-without-any-cardholder-name", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CardHolderNameMissingBoth"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        utils.should_continue_further(data);
      });
    }
  );
});
