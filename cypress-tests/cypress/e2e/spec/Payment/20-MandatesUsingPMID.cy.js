import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let originalCustomerId;

// Performs a raw MIT confirm using a saved payment_method_id and returns the
// response. Needed because `mitUsingPMId` hardcodes failure-shape assertions
// (e.g. `connector_transaction_id` must be null on failure) that contradict
// Stripe's actual error responses, which carry the PaymentIntent id
// (see crates/hyperswitch_connectors/src/connectors/stripe.rs build_error_response).
function mitRawConfirm(globalState, data) {
  const { Request: reqData } = data;
  const requestBody = JSON.parse(JSON.stringify(fixtures.pmIdConfirmBody));

  for (const key in reqData) {
    requestBody[key] = reqData[key];
  }

  requestBody.confirm = true;
  requestBody.capture_method = "automatic";
  requestBody.customer_id = globalState.get("customerId");
  requestBody.profile_id = globalState.get("profileId");
  requestBody.recurring_details.data = globalState.get("paymentMethodId");

  globalState.set("paymentAmount", requestBody.amount);

  return cy
    .request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/payments`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
      body: requestBody,
    })
    .then((response) => {
      if (response.status === 200) {
        globalState.set("paymentID", response.body.payment_id);
      }
      return cy.wrap(response);
    });
}

describe("Card - Mandates using Payment Method Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      originalCustomerId = globalState.get("customerId");
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("restore customerId", () => {
    // Most tests in this spec intentionally continue using a customer
    // created by an earlier test in the same file, so customerId can't be
    // restored after every test. Restore it only once, after the whole
    // spec finishes, so later specs don't inherit a customer scoped to
    // this spec's own tests.
    globalState.set("customerId", originalCustomerId);
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create and Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("customer-create-call-test -> Create No 3DS Payment Intent -> Confirm No 3DS CIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("customer-create-call-test", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create No 3DS Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create No 3DS Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentOffSession"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Create No 3DS Payment Intent -> Confirm No 3DS CIT -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Create No 3DS Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentOffSession"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSManualCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "manual",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("Confirm No 3DS CIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Confirm No 3DS CIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Confirm No 3DS CIT -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT 1 -> mit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT 2 -> mit-capture-call-test -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Confirm No 3DS CIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSManualCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "manual",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT 1", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT 1");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT 2", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT 2");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITManualCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context("Card - MIT without billing address", () => {
    it("Create No 3DS Payment Intent -> Confirm No 3DS CIT -> Confirm No 3DS MIT", () => {
      let shouldContinue = true;

      cy.step("Create No 3DS Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm No 3DS CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm No 3DS MIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITWithoutBillingAddress"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          true,
          "automatic",
          globalState
        );
      });
    });
  });

  context(
    "Card - NoThreeDS MIT with PMID and customer_acceptance flow test",
    () => {
      it("Create Customer -> Create No 3DS Payment Intent -> Confirm No 3DS CIT -> List Customer Payment Methods -> MIT with PMID and customer_acceptance -> List Customer Payment Methods", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create No 3DS Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create No 3DS Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentOffSession"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step("MIT with PMID and customer_acceptance", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: MIT with PMID and customer_acceptance"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCaptureWithCustomerAcceptance"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("Confirm 3DS CIT -> Handle redirection -> retrieve-payment-call-test -> Confirm No 3DS MIT -> Confirm No 3DS MIT", () => {
        let shouldContinue = true;

        cy.step("Confirm 3DS CIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );
        });
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow",
    () => {
      it("Confirm 3DS CIT -> Handle redirection -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT", () => {
        let shouldContinue = true;

        cy.step("Confirm 3DS CIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSManualCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "manual",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );
        });
      });
    }
  );

  context(
    "Card - NoThreeDS CIT and MIT payment with error_on_requires_action metadata flow test",
    () => {
      beforeEach(function () {
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.ERROR_ON_REQUIRES_ACTION
          )
        ) {
          this.skip();
        }
      });

      it("customer-create-call-test -> Confirm No 3DS CIT -> retrieve-payment-call-test -> Confirm No 3DS MIT with error_on_requires_action -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("customer-create-call-test", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT with error_on_requires_action", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm No 3DS MIT with error_on_requires_action"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITWithErrorOnRequiresAction"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITWithErrorOnRequiresAction"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - 3DS CIT and MIT declined by error_on_requires_action flow test",
    () => {
      beforeEach(function () {
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.ERROR_ON_REQUIRES_ACTION
          )
        ) {
          this.skip();
        }
      });

      it("customer-create-call-test -> Confirm 3DS CIT (auth-required card) -> Handle redirection -> retrieve-payment-call-test -> Confirm MIT without metadata -> Confirm MIT with error_on_requires_action -> differential assertions", () => {
        let shouldContinue = true;
        let baselineMitStatus;

        cy.step("customer-create-call-test", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        // CIT uses `successfulThreeDSTestCardDetails` = 4000002500003155,
        // Stripe's card that requires authentication on every transaction.
        cy.step("Confirm 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm 3DS CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });

        cy.step("Confirm No 3DS MIT without error_on_requires_action", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm No 3DS MIT without error_on_requires_action"
            );
            return;
          }

          if (globalState.get("mandateSetupFutureUsage") !== "off_session") {
            cy.task(
              "cli_log",
              "Skipping MIT steps: CIT's setup_future_usage was not 'off_session'"
            );
            shouldContinue = false;
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITWithoutErrorOnRequiresAction"];

          mitRawConfirm(globalState, data).then((response) => {
            expect(response.status, "http status").to.equal(200);
            expect(response.body.payment_id, "payment_id").to.not.be.empty;
            // A plain off_session MIT on this card must never fail outright;
            // Stripe either lets it through (`succeeded`) or asks for
            // authentication (`requires_customer_action`).
            expect(response.body.status, "payment status").to.be.oneOf([
              "requires_customer_action",
              "succeeded",
            ]);
            baselineMitStatus = response.body.status;
          });
        });

        cy.step("Confirm No 3DS MIT with error_on_requires_action", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm No 3DS MIT with error_on_requires_action"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITWithErrorOnRequiresActionFailure"];

          // No cy.then() wrapper needed: the queued commands from the previous
          // step are guaranteed to have completed (baselineMitStatus is set).
          mitRawConfirm(globalState, data).then((response) => {
            expect(response.status, "http status").to.equal(200);

            if (baselineMitStatus === "requires_customer_action") {
              // The baseline MIT needed customer action, so the flagged MIT
              // must be declined outright instead of asking for action.
              // Stripe answers the confirm with HTTP 402 +
              // `payment_intent_authentication_failure`; Hyperswitch maps
              // that to `failed` and surfaces the PaymentIntent id.
              expect(response.body.status, "payment status").to.equal("failed");
              expect(response.body.error_code, "error_code").to.equal(
                "payment_intent_authentication_failure"
              );
              expect(response.body.error_message, "error_message").to.be.a(
                "string"
              ).and.not.be.empty;
              expect(
                response.body.connector_transaction_id,
                "connector_transaction_id"
              ).to.match(/^pi_/);
            } else {
              // Stripe's sandbox did not require authentication for the
              // baseline MIT, so the flag must stay inert on the happy path.
              expect(response.body.status, "payment status").to.equal(
                "succeeded"
              );
              cy.task(
                "cli_log",
                "Stripe sandbox did not require customer action for the baseline MIT on the saved card; strict error_on_requires_action decline assertions were not exercised"
              );
            }
          });
        });
      });
    }
  );
});
