import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

// merchant config | no header | client | server
// unset (default) |    200    |  200   |  422   <- behaves as "client", not "client_and_server"
// client           |    200    |  200   |  422
// server           |    422    |  422   |  200
// client_and_server|    200    |  200   |  200
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

describe("X-Integration-Type header validation against merchant integration_type", () => {
  // Stable payment for every "update intent" case below to target.
  let baselinePaymentId;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  before("reset merchant integration_type", () => {
    cy.deleteMerchantIntegrationType(globalState);
  });

  before("create baseline payment", () => {
    cy.createPaymentIntentWithIntegrationTypeHeader(
      {
        ...fixtures.createPaymentBody,
        amount: 6540,
        confirm: false,
        profile_id: globalState.get("profileId"),
        customer_id: globalState.get("customerId"),
      },
      globalState,
      { headerValue: undefined, expectedStatus: 200 }
    ).then((response) => {
      baselinePaymentId = response.body.payment_id;
    });
  });

  after("flush global state", () => {
    cy.deleteMerchantIntegrationType(globalState);
    cy.task("setGlobalState", globalState.data);
  });

  MATRIX.forEach(({ merchantConfig, header, expectedStatus }) => {
    const label = `merchant=${merchantConfig ?? "unset"} header=${header ?? "none"} -> ${expectedStatus}`;

    context(label, () => {
      before("apply merchant integration_type config", () => {
        if (merchantConfig) {
          cy.setMerchantIntegrationType(globalState, merchantConfig);
        } else {
          cy.deleteMerchantIntegrationType(globalState);
        }
      });

      after("clean up merchant integration_type config", () => {
        cy.deleteMerchantIntegrationType(globalState);
      });

      it(`create intent: ${label}`, () => {
        cy.createPaymentIntentWithIntegrationTypeHeader(
          {
            ...fixtures.createPaymentBody,
            amount: 6540,
            confirm: false,
            profile_id: globalState.get("profileId"),
            customer_id: globalState.get("customerId"),
          },
          globalState,
          { headerValue: header, expectedStatus }
        );
      });

      it(`update intent: ${label}`, () => {
        cy.updatePaymentWithIntegrationTypeHeader(
          baselinePaymentId,
          { amount: 7000, currency: "USD" },
          globalState,
          { headerValue: header, expectedStatus }
        );
      });
    });
  });
});
