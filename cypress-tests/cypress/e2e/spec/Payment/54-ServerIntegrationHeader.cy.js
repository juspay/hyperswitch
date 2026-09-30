import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

// Superposition context the `system.payment_integration_type` config is
// dimensioned on (see dimension_config.rs: processor + provider merchant id).
const merchantContext = () => ({
  provider_merchant_id: globalState.get("merchantId"),
  processor_merchant_id: globalState.get("merchantId"),
});

// Each stage sets the exact `system.payment_integration_type` value it asserts
// rather than relying on the environment default — the spec then validates the
// gate (`client`), the opt-in (`client_and_server`) and the strict mode
// (`server`) regardless of what the workspace default-config holds.
describe("X-Integration-Type: server — intent enrichment (superposition gated)", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      specShouldSkip = shouldIncludeConnector(
        globalState.get("connectorId"),
        CONNECTOR_LISTS.INCLUDE.SERVER_INTEGRATION_HEADER
      );
      if (specShouldSkip) {
        cy.task(
          "cli_log",
          "Connector not in SERVER_INTEGRATION_HEADER list — skipping ServerIntegrationHeader spec"
        );
        return;
      }
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping ServerIntegrationHeader spec"
        );
        specShouldSkip = true;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup superposition config + flush global state", () => {
    // Best effort: drop the merchant's override context so the merchant falls
    // back to the workspace default — every stage sets its value explicitly,
    // so the stage outcomes never depend on what the default is.
    cy.deleteSuperpositionConfig(globalState, merchantContext());
    cy.task("setGlobalState", globalState.data);
  });

  context("integration type client — server header rejected", () => {
    it("set system.payment_integration_type=client", () => {
      cy.setSuperpositionConfig(
        globalState,
        "system.payment_integration_type",
        "client",
        merchantContext()
      );
    });

    it("wait for config propagation (server header rejected)", () => {
      // Under `client_and_server`/`server` the header-carrying poll returns
      // 200; it flips to 422 once `client` propagates.
      cy.waitForConfigPropagation(
        globalState,
        422,
        "payment_integration_type",
        { "X-Integration-Type": "server" }
      );
    });

    it("create intent with X-Integration-Type: server is rejected (IR_06)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreateRejected"];
      // A 422 is expected here — do not gate on should_continue_further.
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        "server"
      );
    });
  });

  context("integration type client_and_server — opt-in via header", () => {
    it("set system.payment_integration_type=client_and_server", () => {
      cy.setSuperpositionConfig(
        globalState,
        "system.payment_integration_type",
        "client_and_server",
        merchantContext()
      );
    });

    it("wait for config propagation", () => {
      // The previous stage left the merchant as `client` (poll 422); the poll
      // flips to 200 once client_and_server propagates.
      cy.waitForConfigPropagation(
        globalState,
        200,
        "payment_integration_type",
        { "X-Integration-Type": "server" }
      );
    });

    it("create intent with X-Integration-Type: server attaches payment_method_list + session_tokens", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreate"];
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        "server"
      );
    });

    it("update intent with X-Integration-Type: server re-attaches both sections (shared module regression)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationUpdate"];
      cy.updatePaymentIntentServerIntegrationTest(data, globalState, "server");
    });

    it("create intent without the header keeps the plain response shape", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreatePlain"];
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        null
      );
    });

    it("create intent with X-Integration-Type: client keeps the plain response shape", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreatePlain"];
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        "client"
      );
    });

    it("create-and-confirm with X-Integration-Type: server keeps the plain response shape", () => {
      // Card data and the expected final status are connector specific; the
      // plain-shape contract keys come from ServerIntegrationCreateAndConfirm.
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      const serverData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreateAndConfirm"];
      if (
        !confirmData?.Request?.payment_method_data ||
        !confirmData?.Response?.body?.status
      ) {
        cy.task(
          "cli_log",
          "Skipping step: No3DSAutoCapture config unavailable for this connector"
        );
        return;
      }

      const data = {
        Configs: confirmData.Configs,
        Request: {
          ...serverData.Request,
          ...confirmData.Request,
          confirm: true,
        },
        Response: {
          status: 200,
          body: {
            ...serverData.Response.body,
            status: confirmData.Response.body.status,
          },
        },
      };
      // Enrichment is skipped by design for create-and-confirm; only the
      // plain shape and the connector-specific final status are asserted.
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        "server"
      );
    });
  });

  context("integration type server — header required", () => {
    it("set system.payment_integration_type=server", () => {
      cy.setSuperpositionConfig(
        globalState,
        "system.payment_integration_type",
        "server",
        merchantContext()
      );
    });

    it("wait for config propagation (missing header now rejected)", () => {
      // An absent header reads as `client`, which mismatches a `server`
      // merchant: the headerless poll flips from 200 to 422 on propagation.
      cy.waitForConfigPropagation(globalState, 422, "payment_integration_type");
    });

    it("create intent without the header is rejected (IR_06)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationHeaderRequired"];
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        null
      );
    });

    it("create intent with X-Integration-Type: server succeeds and attaches both sections", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ServerIntegrationCreate"];
      cy.createPaymentIntentServerIntegrationTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        "server"
      );
    });
  });
});
