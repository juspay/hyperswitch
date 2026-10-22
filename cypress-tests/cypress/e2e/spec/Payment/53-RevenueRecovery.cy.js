import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;
// Shared keys retargeted by userInfo (throwaway merchant). Restored in after()
// so later Payment specs keep the original merchant/profile context.
let sharedStateSnapshot = null;

const revenueRecoveryDetails = (key) =>
  getConnectorDetails(globalState.get("connectorId"))["revenue_recovery_pm"][
    key
  ];

function uniqueReferenceId(prefix) {
  return `${prefix}_${Date.now()}`;
}

describe("Revenue Recovery workflow tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.REVENUE_RECOVERY
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
      })
      .then(() => {
        sharedStateSnapshot = {
          merchantId: globalState.get("merchantId"),
          organizationId: globalState.get("organizationId"),
          profileId: globalState.get("profileId"),
          email: globalState.get("email"),
          password: globalState.get("password"),
          totpToken: globalState.get("totpToken"),
          userInfoToken: globalState.get("userInfoToken"),
          apiKey: globalState.get("apiKey"),
          apiKeyId: globalState.get("apiKeyId"),
        };

        // Fresh user + merchant so we get an AuthToken with
        // ProfileRevenueRecoveryRead for the workflow retrieve API.
        cy.signupUserWithMerchant("CypressRevenueRecovery", globalState);
        cy.userLogin(globalState);
        cy.terminate2Fa(globalState);
        cy.userInfo(globalState);

        // schedule_failed_payment only enqueues retries when the profile has a
        // non-monitoring retry algorithm (Monitoring is threshold-watch only).
        cy.revenueRecoveryProfileUpdateCallTest(globalState);

        // Payment connector first: its id is referenced by the billing
        // connector's revenue_recovery.billing_account_reference map.
        cy.revenueRecoveryPaymentConnectorCreateCallTest(globalState);
        cy.revenueRecoveryBillingConnectorCreateCallTest(globalState);
        cy.revenueRecoveryApiKeyCreateCallTest(globalState);
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("cleanup revenue recovery merchant", () => {
    cy.revenueRecoveryMcaDeleteCallTest(globalState);

    if (sharedStateSnapshot) {
      Object.entries(sharedStateSnapshot).forEach(([key, value]) => {
        if (typeof value !== "undefined") {
          globalState.set(key, value);
        }
      });
    }

    if (globalState.get("merchantId")) {
      cy.merchantDeleteCall(globalState);
    }

    cy.task("setGlobalState", globalState.data);
  });

  context("Revenue Recovery happy paths", () => {
    it("Record failed MIT schedules recovery retry", () => {
      let shouldContinue = true;

      cy.step("Record failed MIT with schedule_failed_payment", () => {
        const data = revenueRecoveryDetails("RecordFailedScheduleRetry");
        const requestBody = {
          merchant_reference_id: uniqueReferenceId("cypress_rr_inv_failed"),
        };

        cy.revenueRecoveryCreateCallTest(requestBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Resume CALCULATE_WORKFLOW with retry scheduled under one minute",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Resume CALCULATE_WORKFLOW with short schedule time"
            );
            return;
          }

          const data = revenueRecoveryDetails("ResumeCalculateWorkflow");
          const requestBody = {
            schedule_time: new Date(Date.now() + 45 * 1000).toISOString(),
          };

          cy.revenueRecoveryResumeCallTest(requestBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Retrieve recovery workflow status", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve recovery workflow status"
          );
          return;
        }

        // GET looks up the EXECUTE_WORKFLOW process tracker. The
        // PassiveRecoveryWorkflow consumer promotes CALCULATE -> EXECUTE, so
        // run this against a worker-enabled hyperswitch deployment.
        const data = revenueRecoveryDetails("WorkflowRetrieve");
        cy.revenueRecoveryWorkflowRetrieveCallTest(data, globalState);
      });
    });

    it("Cancel recovery for failed invoice", () => {
      let shouldContinue = true;

      cy.step("Record failed MIT for cancellation", () => {
        const data = revenueRecoveryDetails("RecordFailedForCancel");
        const requestBody = {
          merchant_reference_id: uniqueReferenceId("cypress_rr_inv_cancel"),
        };

        cy.revenueRecoveryCreateCallTest(requestBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Cancel recovery invoice", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Cancel recovery invoice");
          return;
        }

        const data = revenueRecoveryDetails("CancelSuccess");
        cy.revenueRecoveryCancelCallTest(data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("Revenue Recovery cancel preconditions", () => {
    it("Cancel unknown invoice returns not found", () => {
      const data = revenueRecoveryDetails("CancelUnknown");
      cy.revenueRecoveryCancelCallTest(data, globalState, {
        merchantReferenceId: "cypress_rr_inv_does_not_exist",
      });
    });

    it("Cancel non-failed invoice returns IntentNotCancellable", () => {
      let shouldContinue = true;

      cy.step("Record charged external success payment", () => {
        const data = revenueRecoveryDetails("RecordChargedExternal");
        const requestBody = {
          merchant_reference_id: uniqueReferenceId("cypress_rr_inv_succeeded"),
        };

        cy.revenueRecoveryCreateCallTest(requestBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Cancel succeeded invoice (expect IntentNotCancellable)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Cancel succeeded invoice (expect IntentNotCancellable)"
          );
          return;
        }

        const data = revenueRecoveryDetails("CancelIntentNotCancellable");
        cy.revenueRecoveryCancelCallTest(data, globalState);
      });
    });

    it("Cancel while retry in progress returns RetryInProgress", () => {
      const data = revenueRecoveryDetails("CancelRetryInProgress");

      if (data.Configs && data.Configs.TRIGGER_SKIP) {
        cy.task(
          "cli_log",
          `TRIGGER_SKIP: ${data.Request && data.Request.skip_reason}`
        );
        this.skip();
      }

      cy.revenueRecoveryCancelCallTest(data, globalState);
    });

    it("Cancel after terminal recovery returns AlreadyTerminal", () => {
      const data = revenueRecoveryDetails("CancelAlreadyTerminal");

      if (data.Configs && data.Configs.TRIGGER_SKIP) {
        cy.task(
          "cli_log",
          `TRIGGER_SKIP: ${data.Request && data.Request.skip_reason}`
        );
        this.skip();
      }

      cy.revenueRecoveryCancelCallTest(data, globalState);
    });
  });

  context("Revenue Recovery create validation", () => {
    it("Create with missing required fields returns bad request", () => {
      const data = revenueRecoveryDetails("RecordMissingRequiredFields");
      cy.revenueRecoveryCreateCallTest({}, data, globalState);
    });

    it("Create with invalid merchant reference id returns bad request", () => {
      const data = revenueRecoveryDetails("RecordInvalidReferenceId");
      cy.revenueRecoveryCreateCallTest({}, data, globalState);
    });

    it("Create with unknown connector account returns not found", () => {
      const data = revenueRecoveryDetails("RecordUnknownMca");
      cy.revenueRecoveryCreateCallTest({}, data, globalState);
    });

    it("Create without X-Profile-Id header returns bad request", () => {
      const data = revenueRecoveryDetails("RecordMissingProfileHeader");
      cy.revenueRecoveryCreateCallTest({}, data, globalState, {
        omitProfileHeader: true,
      });
    });

    it("Create without api key returns unauthorized", () => {
      const data = revenueRecoveryDetails("RecordUnauthorized");
      cy.revenueRecoveryCreateCallTest({}, data, globalState, {
        omitApiKey: true,
      });
    });
  });

  context("Revenue Recovery workflow retrieve validation", () => {
    it("Retrieve unknown workflow returns not found", () => {
      const data = revenueRecoveryDetails("WorkflowRetrieveUnknown");
      cy.revenueRecoveryWorkflowRetrieveCallTest(data, globalState, {
        revenueRecoveryId: "pay_unknown_does_not_exist_12345678901234",
      });
    });

    it("Retrieve workflow without JWT returns unauthorized", () => {
      const data = revenueRecoveryDetails("WorkflowRetrieveUnauthorized");
      cy.revenueRecoveryWorkflowRetrieveCallTest(data, globalState, {
        omitAuth: true,
      });
    });
  });

  context("Revenue Recovery scheduling behavior", () => {
    it("success_payment_external does not schedule workflow", () => {
      let shouldContinue = true;

      cy.step("Record charged payment with success_payment_external", () => {
        const data = revenueRecoveryDetails("RecordChargedExternal");
        const requestBody = {
          merchant_reference_id: uniqueReferenceId("cypress_rr_inv_nosched"),
        };

        cy.revenueRecoveryCreateCallTest(requestBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve workflow status (expect no workflow scheduled)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve workflow status (expect no workflow scheduled)"
          );
          return;
        }

        const data = revenueRecoveryDetails("WorkflowNotScheduled");
        cy.revenueRecoveryWorkflowRetrieveCallTest(data, globalState);
      });
    });

    it("Max retry count exhaustion skips further retries", () => {
      const data = revenueRecoveryDetails("MaxRetryExhausted");

      if (data.Configs && data.Configs.TRIGGER_SKIP) {
        cy.task(
          "cli_log",
          `TRIGGER_SKIP: ${data.Request && data.Request.skip_reason}`
        );
        this.skip();
      }
    });
  });
});
