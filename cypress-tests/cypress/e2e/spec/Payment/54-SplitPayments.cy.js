import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Split Payments test", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.SPLIT_PAYMENTS
          )
        ) {
          skip = true;
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

  context("NoThreeDS payment with split payments ledger", () => {
    it("create and confirm payment with split payments -> verify succeeded and ledger echoed back", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCaptureSplitPayment"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) {
        cy.task(
          "cli_log",
          "Skipping remaining assertions: split payment failed"
        );
        return;
      }

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("NoThreeDS payment without split payments (regression)", () => {
    it("create and confirm payment without split payments -> verify succeeded", () => {
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
  });

  context("NoThreeDS CIT and MIT with split payments ledger", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Confirm No 3DS CIT with split payments -> verify succeeded and mandate created", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateSingleUseNo3DSAutoCaptureSplitPayment"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS MIT with split payments -> verify succeeded and ledger echoed back", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCaptureSplitPayment"];

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      // mitForMandatesCallTest asserts resData.body fields with plain
      // `.to.equal`, which cannot verify a nested object like
      // split_payments (strict reference equality). Assert the echoed
      // ledger directly here instead.
      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/payments/${globalState.get(
          "paymentID"
        )}?force_sync=true`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
      }).then((response) => {
        expect(response.body.split_payments).to.deep.equal(
          data.Request.split_payments
        );
      });
    });
  });
});
