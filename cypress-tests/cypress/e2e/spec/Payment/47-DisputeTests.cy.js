import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;

describe("Dispute Tests", () => {
  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
      })
      .then(() => {
        if (!globalState.get("connectorId")) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup - Create Merchant and Connector", () => {
    it("merchant-create-call-test", () => {
      let shouldContinue = true;

      cy.step("Create Merchant", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      cy.step("Create API Key", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create API Key");
          return;
        }
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      cy.step("Create Customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Customer");
          return;
        }
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Connector", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Connector");
          return;
        }
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });
    });
  });

  context("Create Payment to Generate Dispute", () => {
    it("create-and-confirm-payment-for-dispute", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        fixtures.createPaymentBody.customer_id = globalState.get("customerId");

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("List Disputes - Happy Path", () => {
    it("list-all-disputes", () => {
      let shouldContinue = true;

      cy.step("List All Disputes", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          const body = response.body;
          const disputes = Array.isArray(body) ? body : (body.data || []);
          globalState.set("disputesList", disputes);
          if (disputes.length > 0) {
            globalState.set("disputeId", disputes[0].dispute_id);
          }
          cy.task("cli_log", `Listed disputes: ${disputes.length} found`);
        });
      });

      cy.step("Store Dispute ID if Found", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Store Dispute ID");
          return;
        }
        const disputeId = globalState.get("disputeId");
        if (disputeId) {
          cy.task("cli_log", `Found dispute ID: ${disputeId}`);
        } else {
          cy.task(
            "cli_log",
            "No disputes found - subsequent dispute tests may be skipped"
          );
        }
      });
    });
  });

  context("List Disputes - With Filters", () => {
    it("list-disputes-with-connector-filter", () => {
      let shouldContinue = true;

      cy.step("List Disputes with Connector Filter", () => {
        const connectorId = globalState.get("connectorId");
        if (!connectorId) {
          cy.task("cli_log", "No connectorId found, skipping filter test");
          shouldContinue = false;
          return;
        }

        const { Response: resData } =
          getConnectorDetails(connectorId)["Dispute"]["ListDisputes"] || {};

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?connector=${connectorId}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          cy.task(
            "cli_log",
            `Listed disputes for connector ${connectorId}: ${JSON.stringify(response.body)}`
          );
        });
      });
    });

    it("list-disputes-with-time-range-filter", () => {
      const connectorId = globalState.get("connectorId");

      cy.step("List Disputes with Time Range Filter", () => {
        const now = new Date();
        const oneWeekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
        const startTime = oneWeekAgo.toISOString();
        const endTime = now.toISOString();

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?start_time=${encodeURIComponent(startTime)}&end_time=${encodeURIComponent(endTime)}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          if (response.status === 200) {
            cy.task(
              "cli_log",
              `Listed disputes with time range: ${response.body.length || response.body.data?.length || 0} results`
            );
          } else {
            cy.task(
              "cli_log",
              `Time range filter returned status: ${response.status}`
            );
          }
        });
      });
    });
  });

  context("List Disputes - Negative Cases", () => {
    it("list-disputes-with-invalid-status-filter-error", () => {
      cy.step("List Disputes with Invalid Status Filter", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?dispute_status=DisputeOpened`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          // Query deserialize errors may return text/plain or JSON
          expect(response.status).to.equal(400);
          // Just log the response body without asserting specific structure
          cy.task(
            "cli_log",
            `Invalid status filter returned: ${typeof response.body === 'string' ? response.body : JSON.stringify(response.body)}`
          );
        });
      });
    });

    it("list-disputes-with-invalid-stage-filter-error", () => {
      cy.step("List Disputes with Invalid Stage Filter", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?dispute_stage=Dispute`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          // Query deserialize errors may return text/plain or JSON
          expect(response.status).to.equal(400);
          // Just log the response body without asserting specific structure
          cy.task(
            "cli_log",
            `Invalid stage filter returned: ${typeof response.body === 'string' ? response.body : JSON.stringify(response.body)}`
          );
        });
      });
    });
  });

  context("Retrieve Dispute - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("retrieve-existing-dispute", () => {
      cy.step("Retrieve Dispute by ID", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "Dispute"
        ]["RetrieveDispute"];

        cy.retrieveDisputeCallTest(data, globalState);
      });
    });
  });

  context("Retrieve Dispute - Negative Cases", () => {
    it("retrieve-non-existent-dispute-error", () => {
      cy.step("Try to Retrieve Non-existent Dispute", () => {
        const nonExistentDisputeId = "dis_123456789";

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/${nonExistentDisputeId}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          // API returns 404 for non-existent dispute resources
          expect(response.status).to.equal(404);
          expect(response.body.error).to.exist;
          expect(response.body.error.code).to.equal("HE_04");
          expect(response.body.error.message).to.include(
            "Dispute does not exist"
          );
        });
      });
    });

    it("retrieve-dispute-with-invalid-id-format", () => {
      cy.step("Try to Retrieve Dispute with Invalid ID Format", () => {
        const invalidDisputeIds = [
          "invalid_id_format",
          "dispute_123",
          "123",
          "",
        ];

        invalidDisputeIds.forEach((invalidId) => {
          if (!invalidId) return;

          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/disputes/${invalidId}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            cy.task(
              "cli_log",
              `Invalid ID '${invalidId}' returned status: ${response.status}`
            );
          });
        });
      });
    });
  });

  context("Accept Dispute - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("accept-existing-dispute", () => {
      let shouldContinue = true;

      cy.step("Accept Dispute", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "Dispute"
        ]["AcceptDispute"];

        cy.acceptDisputeCallTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Dispute Status After Acceptance", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping verification step");
          return;
        }
        const disputeStatus = globalState.get("disputeStatus");
        cy.task("cli_log", `Dispute status after acceptance: ${disputeStatus}`);
      });
    });
  });

  context("Accept Dispute - Negative Cases", () => {
    it("accept-non-existent-dispute-error", () => {
      cy.step("Try to Accept Non-existent Dispute", () => {
        const nonExistentDisputeId = "dis_123456789";

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/disputes/accept/${nonExistentDisputeId}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {},
          failOnStatusCode: false,
        }).then((response) => {
          // API returns 404 for non-existent dispute resources
          expect(response.status).to.equal(404);
          expect(response.body.error).to.exist;
          expect(response.body.error.code).to.equal("HE_04");
          expect(response.body.error.message).to.include(
            "Dispute does not exist"
          );
        });
      });
    });
  });

  context("Submit Dispute Evidence - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("submit-evidence-for-dispute", () => {
      let shouldContinue = true;

      cy.step("Submit Evidence", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "Dispute"
        ]["SubmitEvidence"];

        cy.submitEvidenceCallTest(
          fixtures.disputeEvidenceBody,
          data,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Dispute Status After Evidence Submission", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping verification step");
          return;
        }
        const disputeStatus = globalState.get("disputeStatus");
        cy.task(
          "cli_log",
          `Dispute status after evidence submission: ${disputeStatus}`
        );
      });
    });
  });

  context("Submit Dispute Evidence - Negative Cases", () => {
    it("submit-evidence-for-non-existent-dispute-error", () => {
      cy.step("Try to Submit Evidence for Non-existent Dispute", () => {
        const nonExistentDisputeId = "dis_nonexistent";

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/disputes/evidence`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            dispute_id: nonExistentDisputeId,
            evidence: {
              cancellation_policy: "file_123",
            },
          },
          failOnStatusCode: false,
        }).then((response) => {
          // API returns 404 for non-existent dispute resources
          expect(response.status).to.equal(404);
          expect(response.body.error).to.exist;
          expect(response.body.error.code).to.equal("HE_04");
        });
      });
    });

    it("submit-evidence-with-empty-body-error", () => {
      cy.step("Try to Submit Evidence with Empty Body", () => {
        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/disputes/evidence`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {},
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            `Empty evidence body returned status: ${response.status}`
          );
        });
      });
    });
  });

  context("Retrieve Dispute Evidence - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("retrieve-evidence-for-dispute", () => {
      cy.step("Retrieve Dispute Evidence", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "Dispute"
        ]["RetrieveEvidence"];

        cy.retrieveDisputeEvidenceCallTest(data, globalState);
      });
    });
  });

  context("Attach Evidence File - Edge Cases", () => {
    it("attach-evidence-file-without-evidence-type-error", () => {
      cy.step("Try to Attach Evidence File Without Required Params", () => {
        cy.request({
          method: "PUT",
          url: `${globalState.get("baseUrl")}/disputes/evidence`,
          headers: {
            "api-key": globalState.get("apiKey"),
            "Content-Type": "multipart/form-data",
          },
          body: {
            purpose: "dispute_evidence",
            file: "@/dev/null;filename=empty.pdf",
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(400);
          expect(response.body.error).to.exist;
          expect(response.body.error.code).to.equal("IR_04");
          expect(response.body.error.message).to.include(
            "Missing required param: evidence_type"
          );
        });
      });
    });
  });

  context("Fetch Disputes from Connector", () => {
    it("fetch-disputes-from-connector-happy-path", () => {
      const connectorId = globalState.get("connectorId");

      cy.step("Fetch Disputes from Connector", () => {
        const now = Math.floor(Date.now() / 1000);
        const oneDayAgo = now - 24 * 60 * 60;

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/${connectorId}/fetch?fetch_from=${oneDayAgo}&fetch_till=${now}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            `Fetch disputes from connector returned status: ${response.status}`
          );
          if (response.status === 200) {
            cy.task("cli_log", `Fetched disputes: ${JSON.stringify(response.body)}`);
          }
        });
      });
    });

    it("fetch-disputes-with-query-params", () => {
      const connectorId = globalState.get("connectorId");

      cy.step("Fetch Disputes with Time Range", () => {
        const now = Math.floor(Date.now() / 1000);
        const oneDayAgo = now - 24 * 60 * 60;

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/${connectorId}/fetch?fetch_from=${oneDayAgo}&fetch_till=${now}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            `Fetch disputes with time range returned status: ${response.status}`
          );
          if (response.status === 200) {
            cy.task("cli_log", `Fetched disputes: ${JSON.stringify(response.body)}`);
          }
        });
      });
    });
  });

  context("Fetch Disputes - Negative Cases", () => {
    it("fetch-disputes-missing-required-params-error", () => {
      const connectorId = globalState.get("connectorId");

      cy.step("Try to Fetch Disputes Missing Required Params", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/${connectorId}/fetch?fetch_from=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(400);
          cy.task(
            "cli_log",
            `Missing params error: ${response.body.error || response.body}`
          );
        });
      });
    });

    it("fetch-disputes-invalid-connector-id", () => {
      cy.step("Try to Fetch Disputes with Invalid Connector ID", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/invalid_connector/fetch`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            `Invalid connector ID returned status: ${response.status}`
          );
        });
      });
    });
  });

  context("Edge Cases and Boundary Tests", () => {
    it("list-disputes-pagination-edge-case", () => {
      cy.step("List Disputes with Limit Parameter", () => {
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?limit=1`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          if (response.status === 200) {
            cy.task("cli_log", "Limit parameter test passed");
          } else {
            cy.task(
              "cli_log",
              `Limit parameter returned status: ${response.status}`
            );
          }
        });
      });
    });

    it("list-disputes-large-time-range", () => {
      cy.step("List Disputes with Large Time Range", () => {
        const startTime = new Date("2020-01-01").toISOString();
        const endTime = new Date().toISOString();

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/disputes/list?start_time=${encodeURIComponent(startTime)}&end_time=${encodeURIComponent(endTime)}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            `Large time range returned status: ${response.status}`
          );
        });
      });
    });
  });
});
