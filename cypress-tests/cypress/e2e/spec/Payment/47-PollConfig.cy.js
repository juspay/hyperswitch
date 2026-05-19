import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Poll Config - Payment status polling", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.POLL_CONFIG
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("3DS payment confirm triggers polling state", () => {
    it("create payment intent -> confirm 3DS payment -> verify polling state", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PollConfig"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("verify poll config - next_action redirect URL exists", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: verify poll config");
          return;
        }
        const nextActionUrl = globalState.get("nextActionUrl");
        expect(nextActionUrl, "next_action redirect URL").to.not.be.undefined;
        expect(nextActionUrl, "next_action redirect URL").to.not.be.empty;
      });
    });
  });

  context(
    "Poll endpoint with valid poll_id returns 200 for 3DS payment",
    () => {
      it("create payment intent -> confirm 3DS -> poll with valid poll_id -> verify 200", () => {
        let shouldContinue = true;

        cy.step("create payment intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("confirm 3DS payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: confirm 3DS payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PollConfig"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("poll with valid poll_id", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: poll with valid poll_id");
            return;
          }
          const paymentID = globalState.get("paymentID");
          const pollId = `external_authentication_${paymentID}`;
          const data = {
            Response: {
              status: 200,
              body: {
                poll_id: pollId,
                status: "pending",
              },
            },
          };

          cy.pollStatusCallTest(pollId, data, globalState, true);
        });
      });
    }
  );

  context(
    "Poll endpoint with constructed poll_id returns 404 when poll_id not in Redis",
    () => {
      it("create payment intent -> confirm 3DS -> poll with constructed poll_id -> verify 404", () => {
        let shouldContinue = true;

        cy.step("create payment intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("confirm 3DS payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: confirm 3DS payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PollConfig"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("poll with constructed poll_id", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: poll with constructed poll_id");
            return;
          }
          const paymentID = globalState.get("paymentID");
          const pollId = `external_authentication_${paymentID}`;
          const data = {
            Response: {
              status: 404,
              body: {
                error: {
                  type: "invalid_request",
                  message: "Poll does not exist in our records",
                  code: "HE_02",
                },
              },
            },
          };

          cy.pollStatusCallTest(pollId, data, globalState, true);
        });
      });
    }
  );

  context("Poll endpoint with invalid poll_id", () => {
    it("poll with invalid poll_id and publishable key returns 404", () => {
      const pollData = {
        Response: {
          status: 404,
          body: {
            error: {
              type: "invalid_request",
              message: "Poll does not exist in our records",
              code: "HE_02",
            },
          },
        },
      };

      cy.pollStatusCallTest("test_poll_invalid", pollData, globalState, true);
    });

    it("poll with invalid poll_id and merchant api key returns 401", () => {
      const pollData = {
        Response: {
          status: 401,
          body: {
            error: {
              type: "invalid_request",
              message: "API key not provided or invalid API key used",
              code: "IR_01",
            },
          },
        },
      };

      cy.pollStatusCallTest("test_poll_invalid", pollData, globalState, false);
    });
  });

  context("Non-3DS payment does not trigger polling state", () => {
    it("create payment intent -> confirm non-3DS payment -> verify success", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
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

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("confirm non-3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm non-3DS payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });
  });

  context("Force sync retrieves 3DS payment status", () => {
    it("create payment intent -> confirm 3DS -> force sync -> verify status", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PollConfig"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("force sync payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: force sync payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
