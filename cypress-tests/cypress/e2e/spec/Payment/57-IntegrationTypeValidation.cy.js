import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

// Matrix from PR #14173 (merchant `system.payment_integration_type` x request
// `X-Integration-Type` header), verified live against integ.hyperswitch.io:
//
//   merchant config      | no header | client | server
//   ---------------------|-----------|--------|-------
//   unset (default)      |    200    |  200   |  422   <- behaves as "client", not "client_and_server"
//   client                |    200    |  200   |  422
//   server                |    422    |  422   |  200
//   client_and_server     |    200    |  200   |  200
//
// An absent header reads as "client". A mismatch returns 422 (IR_06) with
// message: `x-integration-type` header value `<header>` does not match the
// merchant integration type `<merchant>`.
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
  // A single payment created up front, under the default (unset) merchant
  // config with no header — always succeeds regardless of what the matrix
  // sets the merchant config to afterwards, so every "update intent" case
  // below has a stable target to test the header check against.
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
