import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

const MATRIX = [
  { merchantConfig: undefined, header: undefined, expectedStatus: 200 },
  { merchantConfig: undefined, header: "client", expectedStatus: 200 },
  { merchantConfig: undefined, header: "server", expectedStatus: 422 },
  { merchantConfig: "client", header: undefined, expectedStatus: 200 },
  { merchantConfig: "client", header: "client", expectedStatus: 200 },
  { merchantConfig: "client", header: "server", expectedStatus: 422 },
  { merchantConfig: "server", header: undefined, expectedStatus: 422 },
  { merchantConfig: "server", header: "client", expectedStatus: 422 },
  { merchantConfig: "server", header: "server", expectedStatus: 200 },
  {
    merchantConfig: "client_and_server",
    header: undefined,
    expectedStatus: 200,
  },
  {
    merchantConfig: "client_and_server",
    header: "client",
    expectedStatus: 200,
  },
  {
    merchantConfig: "client_and_server",
    header: "server",
    expectedStatus: 200,
  },
];

// requires = DimensionsWithProcessorAndProviderMerchantId (dimension_config.rs)
function merchantIntegrationTypeContext() {
  const merchantId = globalState.get("merchantId");
  return {
    processor_merchant_id: merchantId,
    provider_merchant_id: merchantId,
  };
}

function intentData(expectedStatus) {
  return {
    Request: {
      currency: "USD",
      amount: 6540,
      confirm: false,
    },
    Response: {
      status: expectedStatus,
      body:
        expectedStatus === 200
          ? { status: "requires_payment_method" }
          : { error: { code: "IR_06" } },
    },
  };
}

describe("X-Integration-Type header validation against merchant integration_type", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.deleteSuperpositionContext(
      globalState,
      merchantIntegrationTypeContext()
    );
    cy.task("setGlobalState", globalState.data);
  });

  MATRIX.forEach(({ merchantConfig, header, expectedStatus }) => {
    const label = `merchant=${merchantConfig ?? "unset"} header=${header ?? "none"} -> ${expectedStatus}`;
    let updatePaymentId;

    context(label, () => {
      before("apply merchant integration_type config", () => {
        if (merchantConfig) {
          cy.createSuperpositionConfig(
            globalState,
            "system.payment_integration_type",
            merchantConfig,
            merchantIntegrationTypeContext()
          );
        } else {
          cy.deleteSuperpositionContext(
            globalState,
            merchantIntegrationTypeContext()
          );
        }
      });

      before("create payment for update test", () => {
        // A header value guaranteed to pass under the config just set above,
        // independent of whatever header this scenario tests against — a
        // "server" merchant only accepts a "server" header, everything else
        // accepts no header.
        const validHeader = merchantConfig === "server" ? "server" : undefined;

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          intentData(200),
          "no_three_ds",
          "automatic",
          globalState,
          undefined,
          validHeader
        );

        // createPaymentIntentTest doesn't return its response chain; capture
        // the payment id from globalState now, before the "create intent"
        // test below runs and overwrites it with its own payment.
        cy.then(() => {
          updatePaymentId = globalState.get("paymentID");
        });
      });

      after("clean up merchant integration_type config", () => {
        cy.deleteSuperpositionContext(
          globalState,
          merchantIntegrationTypeContext()
        );
      });

      it(`create intent: ${label}`, () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          intentData(expectedStatus),
          "no_three_ds",
          "automatic",
          globalState,
          undefined,
          header
        );
      });

      it(`update intent: ${label}`, () => {
        cy.updatePaymentWithIntegrationTypeHeader(
          updatePaymentId,
          { amount: 7000, currency: "USD" },
          globalState,
          { headerValue: header, expectedStatus }
        );
      });
    });
  });
});
