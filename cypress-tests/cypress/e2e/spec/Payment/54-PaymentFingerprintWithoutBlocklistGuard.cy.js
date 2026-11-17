import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

const blocklistContext = () => ({
  processor_merchant_id: globalState.get("merchantId"),
  provider_merchant_id: globalState.get("merchantId"),
  profile_id: globalState.get("profileId"),
});

describe("Card - Payment fingerprint without the blocklist guard", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        if (confirmData?.Response?.body?.status !== "succeeded") {
          skip = true;
          return;
        }

        if (
          globalState.get("superpositionBaseUrl") &&
          globalState.get("superpositionSecret") &&
          globalState.get("superpositionAuthToken")
        ) {
          cy.setSuperpositionConfig(
            globalState,
            "payments.payment_blocklist_guard",
            false,
            blocklistContext()
          );
        } else {
          cy.task(
            "cli_log",
            "Superposition credentials not set — relying on the default blocklist guard (off)"
          );
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

  context("Card payment is fingerprinted with the blocklist guard off", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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

        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({
          globalState,
          data: confirmData,
          expectFingerprint: true,
        });
      });
    });
  });
});
