import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

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

  context("List Disputes - Happy Path", () => {
    it("list-all-disputes", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputes"];

      cy.listDisputesCallTest(data, globalState);

      cy.step("Store Dispute ID if Found", () => {
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
      const connectorId = globalState.get("connectorId");
      if (!connectorId) {
        cy.task("cli_log", "No connectorId found, skipping filter test");
        return;
      }

      const data =
        getConnectorDetails(connectorId)["Dispute"][
          "ListDisputesWithConnectorFilter"
        ];

      cy.listDisputesWithFilterCallTest(
        { connector: connectorId },
        data,
        globalState
      );
    });

    it("list-disputes-with-time-range-filter", () => {
      const now = new Date();
      const oneWeekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      const startTime = oneWeekAgo.toISOString();
      const endTime = now.toISOString();

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputesWithTimeRange"];

      cy.listDisputesWithFilterCallTest(
        { start_time: startTime, end_time: endTime },
        data,
        globalState
      );
    });
  });

  context("List Disputes - Negative Cases", () => {
    it("list-disputes-with-invalid-status-filter-error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputesWithInvalidStatusFilter"];

      cy.listDisputesWithFilterCallTest(
        { dispute_status: "DisputeOpened" },
        data,
        globalState
      );
    });

    it("list-disputes-with-invalid-stage-filter-error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputesWithInvalidStageFilter"];

      cy.listDisputesWithFilterCallTest(
        { dispute_stage: "Dispute" },
        data,
        globalState
      );
    });
  });

  context("Retrieve Dispute - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("retrieve-existing-dispute", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["RetrieveDispute"];

      cy.retrieveDisputeCallTest(data, globalState);
    });
  });

  context("Retrieve Dispute - Negative Cases", () => {
    it("retrieve-non-existent-dispute-error", () => {
      const nonExistentDisputeId = "dis_123456789";
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["RetrieveNonExistentDispute"];

      cy.retrieveDisputeByIdCallTest(nonExistentDisputeId, data, globalState);
    });

    it("retrieve-dispute-with-invalid-id-format", () => {
      const invalidDisputeIds = ["invalid_id_format", "dispute_123", "123"];

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["RetrieveNonExistentDispute"];

      invalidDisputeIds.forEach((invalidId) => {
        cy.retrieveDisputeByIdCallTest(invalidId, data, globalState);
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
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["AcceptDispute"];

      cy.acceptDisputeCallTest(data, globalState);

      cy.step("Verify Dispute Status After Acceptance", () => {
        const disputeStatus = globalState.get("disputeStatus");
        cy.task("cli_log", `Dispute status after acceptance: ${disputeStatus}`);
      });
    });
  });

  context("Accept Dispute - Negative Cases", () => {
    it("accept-non-existent-dispute-error", () => {
      const nonExistentDisputeId = "dis_123456789";
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["AcceptNonExistentDispute"];

      cy.acceptDisputeByIdCallTest(nonExistentDisputeId, data, globalState);
    });
  });

  context("Submit Dispute Evidence - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("submit-evidence-for-dispute", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["SubmitEvidence"];

      cy.submitEvidenceCallTest(
        fixtures.disputeEvidenceBody,
        data,
        globalState
      );

      cy.step("Verify Dispute Status After Evidence Submission", () => {
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
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["SubmitEvidenceNonExistentDispute"];

      cy.submitDisputeEvidenceByIdCallTest(
        {
          dispute_id: "dis_nonexistent",
          evidence: {
            cancellation_policy: "file_123",
          },
        },
        data,
        globalState
      );
    });

    it("submit-evidence-with-empty-body-error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["SubmitEvidenceEmptyBody"];

      cy.submitDisputeEvidenceByIdCallTest({}, data, globalState);
    });
  });

  context("Retrieve Dispute Evidence - Conditional on Existing Dispute", () => {
    before(function () {
      if (!globalState.get("disputeId")) {
        this.skip();
      }
    });

    it("retrieve-evidence-for-dispute", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["RetrieveEvidence"];

      cy.retrieveDisputeEvidenceCallTest(data, globalState);
    });
  });

  context("Attach Evidence File - Edge Cases", () => {
    it("attach-evidence-file-without-evidence-type-error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["AttachEvidenceFileMissingType"];

      cy.attachDisputeEvidenceFileCallTest(data, globalState);
    });
  });

  context("Fetch Disputes from Connector", () => {
    it("fetch-disputes-from-connector-happy-path", () => {
      const connectorId = globalState.get("connectorId");
      const now = Math.floor(Date.now() / 1000);
      const oneDayAgo = now - 24 * 60 * 60;

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["FetchDisputes"];

      cy.fetchDisputesFromConnectorCallTest(
        connectorId,
        { fetch_from: oneDayAgo, fetch_till: now },
        data,
        globalState
      );
    });

    it("fetch-disputes-with-query-params", () => {
      const connectorId = globalState.get("connectorId");
      const now = Math.floor(Date.now() / 1000);
      const oneDayAgo = now - 24 * 60 * 60;

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["FetchDisputesWithTimeRange"];

      cy.fetchDisputesFromConnectorCallTest(
        connectorId,
        { fetch_from: oneDayAgo, fetch_till: now },
        data,
        globalState
      );
    });
  });

  context("Fetch Disputes - Negative Cases", () => {
    it("fetch-disputes-missing-required-params-error", () => {
      const connectorId = globalState.get("connectorId");
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["FetchDisputesMissingParams"];

      cy.fetchDisputesFromConnectorCallTest(
        connectorId,
        { fetch_from: "true" },
        data,
        globalState
      );
    });

    it("fetch-disputes-invalid-connector-id", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["FetchDisputesInvalidConnector"];

      cy.fetchDisputesFromConnectorCallTest(
        "invalid_connector",
        {},
        data,
        globalState
      );
    });
  });

  context("Edge Cases and Boundary Tests", () => {
    it("list-disputes-pagination-edge-case", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputesWithLimit"];

      cy.listDisputesWithFilterCallTest({ limit: "1" }, data, globalState);
    });

    it("list-disputes-large-time-range", () => {
      const startTime = new Date("2020-01-01").toISOString();
      const endTime = new Date().toISOString();

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "Dispute"
      ]["ListDisputesWithLargeTimeRange"];

      cy.listDisputesWithFilterCallTest(
        { start_time: startTime, end_time: endTime },
        data,
        globalState
      );
    });
  });
});
